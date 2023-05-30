package wrash

import (
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
	t.Run("Unique", func(t *testing.T) {
		session, err := NewSession("foo", OptionDisablePrompt())
		require.NoError(t, err)

		h := NewHistory(session, []*Entry{}).(*history)
		assert.Equal(t, []*Entry{
			{
				Base: session.Base,
			},
		}, h.entries)

		h.Add("bar")
		assert.Equal(t, []*Entry{
			{
				Base: session.Base,
				Cmd:  "bar",
			},
			{
				Base: session.Base,
			},
		}, h.entries)

		h.Add("!!help")
		assert.Equal(t, []*Entry{
			{
				Base: session.Base,
				Cmd:  "bar",
			},
			{
				Cmd: "!!help",
			},
			{
				Base: session.Base,
			},
		}, h.entries)
	})

	t.Run("DuplicateSameBase", func(t *testing.T) {
		session, err := NewSession("foo", OptionDisablePrompt())
		require.NoError(t, err)

		h := NewHistory(session, []*Entry{
			{
				Base: "foo",
				Cmd:  "a",
			},
		}).(*history)
		assert.Equal(t, []*Entry{
			{
				Base: session.Base,
				Cmd:  "a",
			},
			{
				Base: session.Base,
			},
		}, h.entries)

		h.Add("a")
		assert.Equal(t, []*Entry{
			{
				Base: session.Base,
				Cmd:  "a",
			},
			{
				Base: session.Base,
			},
		}, h.entries)
	})

	t.Run("DuplicateDifferentBase", func(t *testing.T) {
		session, err := NewSession("foo", OptionDisablePrompt())
		require.NoError(t, err)

		h := NewHistory(session, []*Entry{
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
				Base: session.Base,
				Cmd:  "a",
			},
			{
				Base: "bar",
				Cmd:  "a",
			},
			{
				Base: session.Base,
			},
		}, h.entries)

		h.Add("a")
		assert.Equal(t, []*Entry{
			{
				Base: session.Base,
				Cmd:  "a",
			},
			{
				Base: "bar",
				Cmd:  "a",
			},
			{
				Base: session.Base,
			},
		}, h.entries)
	})
}

func TestHistoryOlder(t *testing.T) {
	t.Run("BaseFoo", func(t *testing.T) {
		session, err := NewSession("foo", OptionDisablePrompt())
		require.NoError(t, err)

		h := NewHistory(session, entries)

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
		session, err := NewSession("bar", OptionDisablePrompt())
		require.NoError(t, err)

		h := NewHistory(session, entries)

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
		session, err := NewSession("foo", OptionDisablePrompt())
		require.NoError(t, err)

		h := NewHistory(
			session, []*Entry{
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
		session, err := NewSession("foo", OptionDisablePrompt())
		require.NoError(t, err)

		h := NewHistory(session, entries).(*history)
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
		session, err := NewSession("bar", OptionDisablePrompt())
		require.NoError(t, err)

		h := NewHistory(session, entries).(*history)
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
			&Session{
				Base: "foo",
			}, []*Entry{
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
	session, err := NewSession("foo", OptionDisablePrompt())
	require.NoError(t, err)

	history := NewHistory(session, []*Entry{
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
