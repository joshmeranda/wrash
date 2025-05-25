package wrash

import (
	"io"
	"testing"

	prompt "github.com/joshmeranda/go-prompt"
	"github.com/stretchr/testify/assert"
	"github.com/stretchr/testify/require"
)

var entries = []*Entry{
	{
		Base: "foo",
		Cmd:  "a",
	},
	{
		Base: "bar",
		Cmd:  "a",
	},
	{
		Base: "foo",
		Cmd:  "b",
	},
}

func TestHistoryAdd(t *testing.T) {
	base := "foo"

	t.Run("Unique", func(t *testing.T) {
		h := NewHistory(base, io.Discard, []*Entry{}).(*history)
		assert.Equal(t, []*Entry{
			{
				Base: base,
			},
		}, h.entries)

		h.Add("bar")
		assert.Equal(t, []*Entry{
			{
				Base: base,
				Cmd:  "bar",
			},
			{
				Base: base,
			},
		}, h.entries)

		h.Add("!!help")
		assert.Equal(t, []*Entry{
			{
				Base: base,
				Cmd:  "bar",
			},
			{
				Cmd: "!!help",
			},
			{
				Base: base,
			},
		}, h.entries)
	})

	t.Run("DuplicateSameBase", func(t *testing.T) {
		h := NewHistory(base, io.Discard, []*Entry{
			{
				Base: "foo",
				Cmd:  "a",
			},
		}).(*history)
		assert.Equal(t, []*Entry{
			{
				Base: base,
				Cmd:  "a",
			},
			{
				Base: base,
			},
		}, h.entries)

		h.Add("a")
		assert.Equal(t, []*Entry{
			{
				Base: base,
				Cmd:  "a",
			},
			{
				Base: base,
			},
		}, h.entries)
	})

	t.Run("DuplicateDifferentBase", func(t *testing.T) {
		h := NewHistory(base, io.Discard, []*Entry{
			{
				Base: "foo",
				Cmd:  "a",
			},
			{
				Base: "bar",
				Cmd:  "a",
			},
		}).(*history)
		assert.Equal(t, []*Entry{
			{
				Base: base,
				Cmd:  "a",
			},
			{
				Base: "bar",
				Cmd:  "a",
			},
			{
				Base: base,
			},
		}, h.entries)

		h.Add("a")
		assert.Equal(t, []*Entry{
			{
				Base: base,
				Cmd:  "a",
			},
			{
				Base: "bar",
				Cmd:  "a",
			},
			{
				Base: base,
			},
		}, h.entries)
	})
}

func TestHistoryOlder(t *testing.T) {
	t.Run("BaseFoo", func(t *testing.T) {
		h := NewHistory("foo", io.Discard, entries)

		buf := prompt.NewBuffer()
		buf.InsertText("", false, true)

		older, ok := h.Older(buf)
		require.True(t, ok)
		require.Equal(t, "b", older.Text())

		older, ok = h.Older(buf)
		require.True(t, ok)
		require.Equal(t, "a", older.Text())

		older, ok = h.Older(buf)
		require.False(t, ok)
		require.Equal(t, "", older.Text())
	})

	t.Run("BaseBar", func(t *testing.T) {
		h := NewHistory("bar", io.Discard, entries)

		buf := prompt.NewBuffer()
		buf.InsertText("", false, true)

		older, ok := h.Older(buf)
		require.True(t, ok)
		require.Equal(t, "a", older.Text())

		older, ok = h.Older(buf)
		require.False(t, ok)
		require.Equal(t, "", older.Text())
	})

	t.Run("BaseFooWithChanges", func(t *testing.T) {
		h := NewHistory(
			"foo", io.Discard, []*Entry{
				{
					Base:    "foo",
					Cmd:     "a",
					changes: "A",
				},
			},
		)

		buf := prompt.NewBuffer()
		buf.InsertText("", false, true)

		older, ok := h.Older(buf)
		require.True(t, ok)
		require.Equal(t, "A", older.Text())
	})
}

func TestHistoryNewer(t *testing.T) {
	t.Run("WrappedCommandFoo", func(t *testing.T) {
		h := NewHistory("foo", io.Discard, entries).(*history)
		h.current = 0

		buf := prompt.NewBuffer()
		buf.InsertText("", false, true)

		newer, ok := h.Newer(buf)
		require.True(t, ok)
		require.Equal(t, "b", newer.Text())

		newer, ok = h.Newer(buf)
		require.True(t, ok)
		require.Equal(t, "", newer.Text())

		newer, ok = h.Newer(buf)
		require.False(t, ok)
		require.Equal(t, "", newer.Text())
	})

	t.Run("WrappedCommandBar", func(t *testing.T) {
		h := NewHistory("bar", io.Discard, entries).(*history)
		h.current = 0

		buf := prompt.NewBuffer()
		buf.InsertText("", false, true)

		newer, ok := h.Newer(buf)
		require.True(t, ok)
		require.Equal(t, "a", newer.Text())

		newer, ok = h.Newer(buf)
		require.True(t, ok)
		require.Equal(t, "", newer.Text())

		newer, ok = h.Newer(buf)
		require.False(t, ok)
		require.Equal(t, "", newer.Text())
	})

	t.Run("BaseFooWithChanges", func(t *testing.T) {
		h := NewHistory(
			"foo", io.Discard, []*Entry{
				{},
				{
					Base:    "foo",
					Cmd:     "a",
					changes: "A",
				},
			},
		).(*history)
		h.current = 0

		buf := prompt.NewBuffer()
		buf.InsertText("", false, true)

		older, ok := h.Newer(buf)
		require.True(t, ok)
		require.Equal(t, "A", older.Text())
	})
}

func TestHistoryFullTraverse(t *testing.T) {
	history := NewHistory("foo", io.Discard, []*Entry{
		{
			Base:    "foo",
			Cmd:     "a",
			changes: "xyz",
		},
		{
			Base: "bar",
			Cmd:  "a",
		},
		{
			Base: "foo",
			Cmd:  "b",
		},
	})

	buf := prompt.NewBuffer()
	buf.InsertText("abc", false, true)

	older, found := history.Older(buf)
	assert.True(t, found)
	assert.Equal(t, "b", older.Text())

	older, found = history.Older(older)
	assert.True(t, found)
	assert.Equal(t, "xyz", older.Text())

	older, found = history.Older(older)
	assert.False(t, found)
	assert.Equal(t, "xyz", older.Text())

	newer, found := history.Newer(older)
	assert.True(t, found)
	assert.Equal(t, "b", newer.Text())

	newer, found = history.Newer(newer)
	assert.True(t, found)
	assert.Equal(t, "abc", newer.Text())
}
