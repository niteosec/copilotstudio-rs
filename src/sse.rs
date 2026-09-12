//! Incremental Server-Sent Events parser.
//!
//! The reference clients lean on platform parsers (`System.Net.ServerSentEvents`, `eventsource-client`,
//! aiohttp line iteration). This is a small WHATWG-style parser that accepts byte chunks as they
//! arrive and yields events as soon as a blank line completes them, so streamed `typing` chunks
//! surface live.
//!
//! Spec behaviours kept: `\n`, `\r\n` and `\r` line endings; `:` comment lines; one leading space
//! stripped from field values; multi-line `data` joined with `\n`; `retry` and unknown fields
//! ignored; a leading UTF-8 BOM skipped. [`SseEvent::id`] is the `id:` field of *that* event block
//! (`None` when absent) — what the Python client and the JS client's `eventsource-client` report —
//! while [`SseParser::last_event_id`] is the persisting last-event-id buffer used for resumption.
//! One deliberate superset: [`SseParser::finish`] emits a trailing event that was not terminated
//! by a blank line (matches the Python client's line-based reader; see `docs/DESIGN.md` D10).

/// One parsed SSE event.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SseEvent {
    /// The `event:` field, `None` when absent (the spec default is `message`).
    pub event: Option<String>,
    /// The concatenated `data:` lines, joined with `\n`.
    pub data: String,
    /// The `id:` field carried by this event block, `None` when it had none.
    pub id: Option<String>,
}

/// Incremental parser state. Feed chunks with [`feed`](Self::feed), flush with [`finish`](Self::finish).
#[derive(Debug, Default)]
pub struct SseParser {
    buf: Vec<u8>,
    event_type: Option<String>,
    data: Option<String>,
    event_id: Option<String>,
    last_id: Option<String>,
    started: bool,
    pending_lf: bool,
}

impl SseParser {
    /// A fresh parser.
    pub fn new() -> Self {
        Self::default()
    }

    /// Consume a chunk and return every event completed by it, in order.
    pub fn feed(&mut self, chunk: &[u8]) -> Vec<SseEvent> {
        let mut chunk = chunk;
        if !self.started {
            self.started = true;
            if let Some(rest) = chunk.strip_prefix(b"\xEF\xBB\xBF") {
                chunk = rest;
            }
        }
        if self.pending_lf {
            self.pending_lf = false;
            if let Some(rest) = chunk.strip_prefix(b"\n") {
                chunk = rest;
            }
        }
        self.buf.extend_from_slice(chunk);

        let mut events = Vec::new();
        while let Some(pos) = self.buf.iter().position(|&b| b == b'\n' || b == b'\r') {
            let line = String::from_utf8_lossy(&self.buf[..pos]).into_owned();
            let mut consumed = pos + 1;
            if self.buf[pos] == b'\r' {
                match self.buf.get(pos + 1) {
                    Some(b'\n') => consumed += 1,
                    Some(_) => {}
                    None => self.pending_lf = true,
                }
            }
            self.buf.drain(..consumed);
            if let Some(event) = self.process_line(&line) {
                events.push(event);
            }
        }
        events
    }

    /// Flush at end of stream: an unterminated final line is processed, and a pending event with
    /// data is dispatched even without its terminating blank line.
    pub fn finish(&mut self) -> Option<SseEvent> {
        if !self.buf.is_empty() {
            let line = String::from_utf8_lossy(&self.buf).into_owned();
            self.buf.clear();
            if let Some(event) = self.process_line(&line) {
                return Some(event);
            }
        }
        self.dispatch()
    }

    /// The last-event-id seen so far (what a resumption would send as `Last-Event-ID`).
    pub fn last_event_id(&self) -> Option<&str> {
        self.last_id.as_deref()
    }

    fn process_line(&mut self, line: &str) -> Option<SseEvent> {
        if line.is_empty() {
            return self.dispatch();
        }
        if line.starts_with(':') {
            return None;
        }
        let (field, value) = match line.split_once(':') {
            Some((f, v)) => (f, v.strip_prefix(' ').unwrap_or(v)),
            None => (line, ""),
        };
        match field {
            "event" => self.event_type = Some(value.to_owned()),
            "data" => match &mut self.data {
                Some(data) => {
                    data.push('\n');
                    data.push_str(value);
                }
                None => self.data = Some(value.to_owned()),
            },
            "id" if !value.contains('\0') => {
                self.event_id = Some(value.to_owned());
                self.last_id = Some(value.to_owned());
            }
            _ => {}
        }
        None
    }

    fn dispatch(&mut self) -> Option<SseEvent> {
        let event = self.event_type.take();
        let id = self.event_id.take();
        self.data.take().map(|data| SseEvent { event, data, id })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn collect(chunks: &[&str]) -> Vec<SseEvent> {
        let mut p = SseParser::new();
        let mut out = Vec::new();
        for c in chunks {
            out.extend(p.feed(c.as_bytes()));
        }
        out.extend(p.finish());
        out
    }

    #[test]
    fn basic_event() {
        let ev = collect(&["event: activity\ndata: {\"a\":1}\n\n"]);
        assert_eq!(ev, vec![SseEvent { event: Some("activity".into()), data: "{\"a\":1}".into(), id: None }]);
    }

    #[test]
    fn multiline_data_and_comments() {
        let ev = collect(&[": hello\nevent: activity\ndata: line1\ndata:line2\n\n"]);
        assert_eq!(ev[0].data, "line1\nline2");
    }

    #[test]
    fn chunk_boundaries_split_lines_and_crlf() {
        let ev = collect(&["eve", "nt: act", "ivity\r", "\ndata: {\"x\"", ":2}\r\n\r\n"]);
        assert_eq!(ev.len(), 1);
        assert_eq!(ev[0].event.as_deref(), Some("activity"));
        assert_eq!(ev[0].data, "{\"x\":2}");
    }

    #[test]
    fn cr_at_chunk_end_then_lf_is_one_terminator() {
        let ev = collect(&["data: a\r", "\n\r", "\n"]);
        assert_eq!(ev, vec![SseEvent { event: None, data: "a".into(), id: None }]);
    }

    #[test]
    fn id_is_per_event_while_last_event_id_persists() {
        let mut p = SseParser::new();
        let ev = p.feed(b"id: 1\nevent: activity\ndata: a\n\nevent: activity\ndata: b\n\nid: 2\ndata: c\n\n");
        assert_eq!(ev.iter().map(|e| e.id.as_deref()).collect::<Vec<_>>(), vec![Some("1"), None, Some("2")]);
        assert_eq!(p.last_event_id(), Some("2"));
    }

    #[test]
    fn blank_line_without_data_resets_type_and_emits_nothing() {
        let ev = collect(&["event: activity\n\ndata: x\n\n"]);
        assert_eq!(ev, vec![SseEvent { event: None, data: "x".into(), id: None }]);
    }

    #[test]
    fn eof_flushes_unterminated_event() {
        let ev = collect(&["event: activity\ndata: tail"]);
        assert_eq!(ev.len(), 1);
        assert_eq!(ev[0].data, "tail");
    }

    #[test]
    fn bom_is_skipped() {
        let ev = collect(&["\u{FEFF}data: x\n\n"]);
        assert_eq!(ev[0].data, "x");
    }
}
