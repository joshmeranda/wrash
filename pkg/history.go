package wrash

import (
	"fmt"
	"io"
	"os"

	prompt "github.com/joshmeranda/go-prompt"
	"gopkg.in/yaml.v3"
)

type Entry struct {
	Base string
	Cmd  string

	changes string
}

type history struct {
	entries []*Entry

	current int
	base    string

	w io.Writer
}

type WriterFunc func([]byte) (int, error)

func (f WriterFunc) Write(b []byte) (int, error) {
	return f(b)
}

func NewHistoryWriter(path string) io.Writer {
	return WriterFunc(func(b []byte) (int, error) {
		if err := os.WriteFile(path, b, 0666); err != nil {
			return 0, fmt.Errorf("could not sync history: %w", err)
		}

		return 0, nil
	})
}

func NewHistory(base string, w io.Writer, entries []*Entry) prompt.History {
	newEntries := make([]*Entry, len(entries), len(entries)+1)
	copy(newEntries, entries)
	newEntries = append(newEntries, &Entry{
		Base: base,
	})

	return &history{
		entries: newEntries,

		current: len(newEntries) - 1,
		base:    base,

		w: w,
	}
}

func (h *history) Add(inputs ...string) {
	h.entries = h.entries[:len(h.entries)-1]

	var lastEntry *Entry
	if len(h.entries) >= 1 {
		lastEntry = h.entries[len(h.entries)-1]
	}

	for _, s := range inputs {
		if s == "" || lastEntry != nil && s == lastEntry.Cmd {
			continue
		}

		base := h.base
		if isBuiltin(s) {
			base = ""
		}

		h.entries = append(h.entries, &Entry{
			Base: base,
			Cmd:  s,
		})
	}
	h.entries = append(h.entries, &Entry{
		Base: h.base,
	})
}

func (h *history) Clear() {
	h.current = len(h.entries) - 1
	for _, entry := range h.entries {
		entry.changes = ""
	}
}

func (h *history) nextOlder(text string) (*Entry, bool) {
	if len(h.entries) == 1 || h.current == 0 {
		return nil, false
	}

	entry := h.entries[h.current]
	if entry.Cmd != text {
		entry.changes = text
	}

	h.current--
	entry = h.entries[h.current]

	return entry, true
}

func (h *history) Older(buf *prompt.Buffer) (*prompt.Buffer, bool) {
	for next, ok := h.nextOlder(buf.Text()); ok; next, ok = h.nextOlder(buf.Text()) {
		if next.Base == h.base || isBuiltin(next.Cmd) {
			var text string
			if next.changes != "" {
				text = next.changes
			} else {
				text = next.Cmd
			}

			new := prompt.NewBuffer()
			new.InsertText(text, false, true)

			return new, true
		}
	}

	return buf, false
}

func (h *history) nextNewer(text string) (*Entry, bool) {
	if h.current == len(h.entries)-1 {
		return nil, false
	}

	entry := h.entries[h.current]
	if entry.Cmd != text {
		entry.changes = text
	}

	h.current++
	entry = h.entries[h.current]

	return entry, true
}

func (h *history) Newer(buf *prompt.Buffer) (*prompt.Buffer, bool) {
	for next, ok := h.nextNewer(buf.Text()); ok; next, ok = h.nextNewer(buf.Text()) {
		if next.Base == h.base || isBuiltin(next.Cmd) {
			var text string
			if next.changes != "" {
				text = next.changes
			} else {
				text = next.Cmd
			}

			new := prompt.NewBuffer()
			new.InsertText(text, false, true)

			return new, true
		}
	}
	fmt.Println()

	return buf, false
}

// todo: read the curent contents and reconcile with the new contents (will want to add some type of ordering mechanism)
func (h *history) Sync() error {
	data, err := yaml.Marshal(h.entries[:len(h.entries)-1])
	if err != nil {
		return fmt.Errorf("could not marshal history entries: %w", err)
	}

	if _, err := h.w.Write(data); err != nil {
		return fmt.Errorf("could not sync history: %w", err)
	}

	return nil
}
