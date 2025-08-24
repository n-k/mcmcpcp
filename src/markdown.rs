// Cargo.toml
// [dependencies]
// tokio = { version = "1", features = ["full"] }
// pulldown-cmark = "0.11"
// futures = "0.3"

// src/main.rs
use futures::FutureExt;
use pulldown_cmark::{html, Options, Parser};
use std::borrow::Cow;
use tokio::io::{self, AsyncBufReadExt, AsyncWrite, AsyncWriteExt, BufReader};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BlockState {
    Idle,
    Paragraph,
    ListUnordered,
    ListOrdered,
    Blockquote,
    FencedCode, // ``` ... ```
}

pub struct StreamMdToHtml {
    state: BlockState,
    buf: String,        // accumulates current block text
    list_open: bool,    // whether <ul>/<ol> is open
    quote_open: bool,   // whether <blockquote> is open
    fence_lang: String, // info string after ```
    pub out: Vec<u8>,
    md_opts: Options,
}

impl StreamMdToHtml {
    pub fn new(out: Vec<u8>) -> Self {
        Self {
            state: BlockState::Idle,
            buf: String::new(),
            list_open: false,
            quote_open: false,
            fence_lang: String::new(),
            out,
            md_opts: Options::ENABLE_TABLES
                | Options::ENABLE_FOOTNOTES
                | Options::ENABLE_STRIKETHROUGH
                | Options::ENABLE_TASKLISTS,
        }
    }

    async fn write(&mut self, s: &str) -> io::Result<()> {
        self.out.write_all(s.as_bytes()).await
    }

    async fn flush_paragraph(&mut self) -> io::Result<()> {
        if self.buf.trim().is_empty() {
            self.buf.clear();
            self.state = BlockState::Idle;
            return Ok(());
        }
        let html_block = render_inline_markdown_to_html(&self.buf, self.md_opts);
        self.write(&html_block).await?;
        self.buf.clear();
        self.state = BlockState::Idle;
        Ok(())
    }

    async fn flush_blockquote(&mut self) -> io::Result<()> {
        if !self.quote_open {
            return Ok(());
        }
        // Render the accumulated quote content as Markdown (so nested formatting works)
        let inner_html = render_inline_markdown_to_html(&self.buf, self.md_opts);
        self.write("<blockquote>\n").await?;
        self.write(&inner_html).await?;
        self.write("</blockquote>\n").await?;
        self.buf.clear();
        self.quote_open = false;
        self.state = BlockState::Idle;
        Ok(())
    }

    async fn ensure_ul_open(&mut self) -> io::Result<()> {
        if !self.list_open || self.state != BlockState::ListUnordered {
            if self.list_open {
                // Close any previously open list (ordered)
                self.write("</ol>\n").await?;
                self.list_open = false;
            }
            self.write("<ul>\n").await?;
            self.list_open = true;
        }
        Ok(())
    }

    async fn ensure_ol_open(&mut self) -> io::Result<()> {
        if !self.list_open || self.state != BlockState::ListOrdered {
            if self.list_open {
                // Close any previously open list (unordered)
                self.write("</ul>\n").await?;
                self.list_open = false;
            }
            self.write("<ol>\n").await?;
            self.list_open = true;
        }
        Ok(())
    }

    async fn close_any_list(&mut self) -> io::Result<()> {
        if self.list_open {
            match self.state {
                BlockState::ListUnordered => self.write("</ul>\n").await?,
                BlockState::ListOrdered => self.write("</ol>\n").await?,
                _ => {}
            }
            self.list_open = false;
        }
        Ok(())
    }

    async fn flush_fenced_code(&mut self) -> io::Result<()> {
        // Simple HTML escape of code content
        let buf = self.buf.clone();
        let escaped = html_escape(&buf);
        if self.fence_lang.trim().is_empty() {
            self.write("<pre><code>").await?;
        } else {
            // Add a language class for client-side highlighters
            self.write(&format!(
                "<pre><code class=\"language-{}\">",
                html_escape(&self.fence_lang.trim())
            ))
            .await?;
        }
        self.write(&escaped).await?;
        self.write("</code></pre>\n").await?;
        self.buf.clear();
        self.fence_lang.clear();
        self.state = BlockState::Idle;
        Ok(())
    }

    async fn flush_all(&mut self) -> io::Result<()> {
        match self.state {
            BlockState::Paragraph => self.flush_paragraph().await?,
            BlockState::Blockquote => self.flush_blockquote().await?,
            BlockState::FencedCode => self.flush_fenced_code().await?,
            BlockState::ListUnordered | BlockState::ListOrdered => {
                // lists don't buffer the whole list; each item is flushed as it arrives
                self.close_any_list().await?;
                self.state = BlockState::Idle;
            }
            BlockState::Idle => {}
        }
        Ok(())
    }

    pub async fn handle_line(&mut self, mut line: String) -> io::Result<()> {
        // Normalize CRLF
        if line.ends_with('\r') {
            line.pop();
        }

        // Handle fenced code block toggling
        if is_fence(&line) {
            match self.state {
                BlockState::FencedCode => {
                    // closing fence
                    self.flush_fenced_code().await?;
                }
                _ => {
                    // opening fence
                    self.state = BlockState::FencedCode;
                    self.fence_lang = fence_info(&line).to_string();
                    self.buf.clear();
                }
            }
            return Ok(());
        }

        if self.state == BlockState::FencedCode {
            // Inside code block: keep raw
            self.buf.push_str(&line);
            self.buf.push('\n');
            return Ok(());
        }

        // Blank line: close paragraphs/quotes/lists as appropriate
        if line.trim().is_empty() {
            match self.state {
                BlockState::Paragraph => self.flush_paragraph().await?,
                BlockState::Blockquote => self.flush_blockquote().await?,
                BlockState::ListUnordered | BlockState::ListOrdered => {
                    self.close_any_list().await?;
                    self.state = BlockState::Idle;
                }
                BlockState::Idle => {}
                BlockState::FencedCode => unreachable!(),
            }
            return Ok(());
        }

        // Headings (ATX)
        if let Some(level) = heading_level(&line) {
            self.flush_all().await?;
            let text = line[level + 1..].trim(); // skip leading #'s and space
            let inner = render_inline_markdown_to_html(text, self.md_opts);
            // inner includes <p> wrappers; strip them for headings
            let inner = strip_paragraph_wrapping(inner);
            self.write(&format!("<h{lvl}>{}</h{lvl}>\n", inner, lvl = level))
                .await?;
            return Ok(());
        }

        // Blockquote lines: start/continue a quote block
        if let Some(rest) = line.strip_prefix('>') {
            let content = rest.strip_prefix(' ').unwrap_or(rest);
            match self.state {
                BlockState::Blockquote => {
                    self.buf.push_str(content);
                    self.buf.push('\n');
                }
                _ => {
                    self.flush_all().await?;
                    self.quote_open = true;
                    self.state = BlockState::Blockquote;
                    self.buf.clear();
                    self.buf.push_str(content);
                    self.buf.push('\n');
                }
            }
            return Ok(());
        } else if self.state == BlockState::Blockquote {
            // A non-quote line ends the quote block
            self.flush_blockquote().await?;
        }

        // List items (unordered)
        if let Some(rest) = is_unordered_item(&line) {
            self.ensure_ul_open().await?;
            self.state = BlockState::ListUnordered;
            let inner = render_inline_markdown_to_html(rest, self.md_opts);
            let inner = strip_paragraph_wrapping(inner);
            self.write("<li>").await?;
            self.write(&inner).await?;
            self.write("</li>\n").await?;
            return Ok(());
        }

        // List items (ordered)
        if let Some(rest) = is_ordered_item(&line) {
            self.ensure_ol_open().await?;
            self.state = BlockState::ListOrdered;
            let inner = render_inline_markdown_to_html(rest, self.md_opts);
            let inner = strip_paragraph_wrapping(inner);
            self.write("<li>").await?;
            self.write(&inner).await?;
            self.write("</li>\n").await?;
            return Ok(());
        }

        // Otherwise, part of a paragraph
        match self.state {
            BlockState::Paragraph => {
                if !self.buf.is_empty() {
                    self.buf.push('\n');
                }
                self.buf.push_str(&line);
            }
            _ => {
                self.flush_all().await?;
                self.state = BlockState::Paragraph;
                self.buf.clear();
                self.buf.push_str(&line);
            }
        }
        Ok(())
    }

    async fn finish(mut self) -> io::Result<Vec<u8>> {
        self.flush_all().await?;
        Ok(self.out)
    }
}

fn render_inline_markdown_to_html(src: &str, opts: Options) -> String {
    // Use pulldown-cmark to render a fragment. It will produce <p>…</p> for paragraphs
    let parser = Parser::new_ext(src, opts);
    let mut out = String::new();
    html::push_html(&mut out, parser);
    out
}

fn strip_paragraph_wrapping(mut html: String) -> String {
    // cheap strip for a single top-level <p>…</p>, if present
    let t = html.trim();
    if t.starts_with("<p>") && t.ends_with("</p>") {
        let inner = &t[3..t.len() - 4];
        inner.to_string()
    } else {
        std::mem::take(&mut html)
    }
}

fn is_fence(line: &str) -> bool {
    let t = line.trim_start();
    t.starts_with("```") || t.starts_with("~~~")
}

fn fence_info(line: &str) -> &str {
    let t = line.trim_start();
    let fence = if t.starts_with("```") { "```" } else { "~~~" };
    t.trim_start_matches(fence).trim()
}

fn heading_level(line: &str) -> Option<usize> {
    let t = line.trim_start();
    let hashes = t.chars().take_while(|&c| c == '#').count();
    if (1..=6).contains(&hashes) && t.chars().nth(hashes) == Some(' ') {
        Some(hashes)
    } else {
        None
    }
}

fn is_unordered_item(line: &str) -> Option<&str> {
    let t = line.trim_start();
    if t.starts_with("- ") || t.starts_with("* ") || t.starts_with("+ ") {
        Some(&t[2..])
    } else {
        None
    }
}

fn is_ordered_item(line: &str) -> Option<&str> {
    let t = line.trim_start();
    // match "1. " / "23) "
    let mut digits = 0usize;
    for ch in t.chars() {
        if ch.is_ascii_digit() {
            digits += 1;
        } else {
            break;
        }
    }
    if digits > 0 && (t.chars().nth(digits) == Some('.') || t.chars().nth(digits) == Some(')'))
        && t.chars().nth(digits + 1) == Some(' ')
    {
        Some(&t[digits + 2..])
    } else {
        None
    }
}

fn html_escape(s: &str) -> Cow<'_, str> {
    if !s.contains(['&', '<', '>', '"', '\'']) {
        return Cow::Borrowed(s);
    }
    let mut out = String::with_capacity(s.len() + 16);
    for ch in s.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(ch),
        }
    }
    Cow::Owned(out)
}

#[tokio::main]
async fn main() -> io::Result<()> {
    // Example: read from stdin line-by-line and write HTML to stdout as soon as possible.
    let stdin = BufReader::new(tokio::io::stdin());
    let mut lines = stdin.lines();

    let stdout = tokio::io::stdout();
    let buffer: Vec<u8> = vec![];
    let mut streamer = StreamMdToHtml::new(buffer);

    while let Some(line) = lines.next_line().map(|r| r.ok()).await.flatten() {
        streamer.handle_line(line).await?;
        // Flush OS buffer so the client sees data immediately
        streamer.out.flush().await?;
    }

    // End of input: close any open blocks and exit
    let mut out = streamer.finish().await?;
    out.flush().await
}
