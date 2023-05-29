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
		Cmd:  "b",
	},
	{},
}

func TestHistoryAdd(t *testing.T) {
	session, err := NewSession("foo", OptionDisablePrompt())
	require.NoError(t, err)

	h := NewHistory(session, []*Entry{}).(*history)
	assert.Equal(t, []*Entry{
		{},
	}, h.entries)

	h.Add("bar")
	assert.Equal(t, []*Entry{
		{
			Base: session.Base,
			Cmd:  "bar",
		},
		{},
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
		{},
	}, h.entries)
}

func TestHistoryOlder(t *testing.T) {
	t.Run("BaseFoo", func(t *testing.T) {
		h := NewHistory(
			&Session{
				Base: "foo",
			}, entries,
		)

		buf := prompt.NewBuffer()
		buf.InsertText("", false, true)

		older, ok := h.Older(buf)
		require.True(t, ok)
		require.Equal(t, "a", older.Text())

		older, ok = h.Older(buf)
		require.False(t, ok)
		require.Equal(t, "", older.Text())
	})

	t.Run("BaseBar", func(t *testing.T) {
		h := NewHistory(
			&Session{
				Base: "bar",
			}, entries,
		)

		buf := prompt.NewBuffer()
		buf.InsertText("", false, true)

		older, ok := h.Older(buf)
		require.True(t, ok)
		require.Equal(t, "b", older.Text())

		older, ok = h.Older(buf)
		require.False(t, ok)
		require.Equal(t, "", older.Text())
	})

	t.Run("BaseFooWithChanges", func(t *testing.T) {
		h := NewHistory(
			&Session{
				Base: "foo",
			}, []*Entry{
				{
					Base:    "foo",
					Cmd:     "a",
					changes: "A",
				},
				{},
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
		h := NewHistory(
			&Session{
				Base: "foo",
			}, entries,
		).(*history)
		h.current = 0

		buf := prompt.NewBuffer()
		buf.InsertText("", false, true)

		newer, ok := h.Newer(buf)
		require.False(t, ok)
		require.Equal(t, "", newer.Text())
	})

	t.Run("WrappedCommandFoo", func(t *testing.T) {
		h := NewHistory(
			&Session{
				Base: "bar",
			}, entries,
		).(*history)
		h.current = 0

		buf := prompt.NewBuffer()
		buf.InsertText("", false, true)

		newer, ok := h.Newer(buf)
		require.True(t, ok)
		require.Equal(t, "b", newer.Text())

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
				{},
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
