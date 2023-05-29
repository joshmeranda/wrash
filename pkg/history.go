package wrash

import (
	"fmt"
	"os"
	"path"

	prompt "github.com/joshmeranda/go-prompt"
	"gopkg.in/yaml.v3"
)

func GetHistoryFile() (string, error) {
	dir, err := os.UserHomeDir()
	if err != nil {
		return "", fmt.Errorf("could not determine user home directory: %w", err)
	}

	return path.Join(dir, ".wrash_history.yaml"), nil
}

type Entry struct {
	Base string
	Cmd  string

	changes string
}

type history struct {
	entries []*Entry

	current int
	session *Session
}

func (h *history) Add(inputs ...string) {
	h.entries = h.entries[:len(h.entries)-1]
	for _, s := range inputs {
		if s == "" {
			continue
		}

		base := h.session.Base
		if isBuiltin(s) {
			base = ""
		}

		h.entries = append(h.entries, &Entry{
			Base: base,
			Cmd:  s,
		})
	}
	h.entries = append(h.entries, &Entry{})
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
		if next.Base == h.session.Base {
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
		if next.Base == h.session.Base {
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

func (h *history) Sync() error {
	data, err := yaml.Marshal(h.entries[:len(h.entries)-1])
	if err != nil {
		return fmt.Errorf("could not marshal history entries: %w", err)
	}

	historyPath, err := GetHistoryFile()
	if err != nil {
		return fmt.Errorf("could not get history file: %w", err)
	}

	if err := os.WriteFile(historyPath, data, 0644); err != nil {
		return fmt.Errorf("could not sync history: %w", err)
	}

	return nil
}

func NewHistory(session *Session, entries []*Entry) prompt.History {
	newEntries := make([]*Entry, len(entries), len(entries)+1)
	copy(newEntries, entries)
	newEntries = append(newEntries, &Entry{})

	return &history{
		entries: newEntries,
		current: len(newEntries) - 1,
		session: session,
	}
}
