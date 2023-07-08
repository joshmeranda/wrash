package wrash

import (
	"os"
	"path/filepath"
	"strings"
	"testing"

	"github.com/stretchr/testify/assert"
	"github.com/stretchr/testify/require"
)

func TestIsBuiltin(t *testing.T) {
	assert.False(t, isBuiltin(""))
	assert.False(t, isBuiltin("!a"))
	assert.False(t, isBuiltin("something"))

	assert.True(t, isBuiltin("!!"))
	assert.True(t, isBuiltin("!!something"))
}

func TestCd(t *testing.T) {
	startDir, err := os.Getwd()
	require.NoError(t, err)
	defer os.Chdir(startDir)

	session, err := NewSession("foo", OptionInteractive(false))
	require.NoError(t, err)

	target, err := filepath.Abs("../tests")
	require.NoError(t, err)

	t.Run("Home", func(t *testing.T) {
		defer os.Chdir(startDir)

		oldHome := os.Getenv("HOME")
		os.Setenv("HOME", target)
		defer os.Setenv("HOME", oldHome)

		require.NoError(t, session.apps["cd"].Run([]string{"!!cd"}))

		dir, err := os.Getwd()
		require.NoError(t, err)
		assert.Equal(t, target, dir)
	})

	t.Run("TargetGiven", func(t *testing.T) {
		defer os.Chdir(startDir)

		require.NoError(t, session.apps["cd"].Run([]string{"!!cd", target}))

		dir, err := os.Getwd()
		require.NoError(t, err)
		assert.Equal(t, target, dir)
	})

	t.Run("TooManyArgs", func(t *testing.T) {
		defer os.Chdir(startDir)

		require.Error(t, session.apps["cd"].Run([]string{"!!cd", target, "another"}))

		dir, err := os.Getwd()
		require.NoError(t, err)
		assert.NotEqual(t, target, dir)
	})

	t.Run("NoExist", func(t *testing.T) {
		defer os.Chdir(startDir)

		require.Error(t, session.apps["cd"].Run([]string{"!!cd", "no-exist"}))
	})
}

func TestExit(t *testing.T) {
	t.Run("NoCodeGiven", func(t *testing.T) {
		session, err := NewSession("foo", OptionInteractive(false))
		require.NoError(t, err)

		require.NoError(t, session.apps["exit"].Run([]string{"!!exit"}))
		assert.True(t, session.exitCalled)
		assert.Equal(t, 0, session.previousExitCode)
	})

	t.Run("CodeGiven", func(t *testing.T) {
		session, err := NewSession("foo", OptionInteractive(false))
		require.NoError(t, err)

		require.NoError(t, session.apps["exit"].Run([]string{"!!exit", "5"}))
		assert.True(t, session.exitCalled)
		assert.Equal(t, 5, session.previousExitCode)
	})

	t.Run("InvalidCodeGiven", func(t *testing.T) {
		session, err := NewSession("foo", OptionInteractive(false))
		require.NoError(t, err)

		require.Error(t, session.apps["exit"].Run([]string{"!!exit", "bad"}))
		assert.False(t, session.exitCalled)
	})

	t.Run("ToomanyArgs", func(t *testing.T) {
		session, err := NewSession("foo", OptionInteractive(false))
		require.NoError(t, err)

		require.Error(t, session.apps["exit"].Run([]string{"!!exit", "1", "2"}))
		assert.False(t, session.exitCalled)
	})
}

func TestHelp(t *testing.T) {
	out := strings.Builder{}
	session, err := NewSession("foo", OptionInteractive(false), OptionStdout(&out))
	require.NoError(t, err)

	require.NoError(t, session.apps["help"].Run([]string{"!!help"}))
	require.NotEmpty(t, out.String())
}

func TestHistory(t *testing.T) {
	session, err := NewSession("foo",
		OptionInteractive(false),
		OptionHistory(NewHistory("foo", sinkWriter{}, []*Entry{
			{
				Base: "foo",
				Cmd:  "bar",
			},
			{
				Base: "foo",
				Cmd:  "baz",
			},
			{
				Base: "bar",
				Cmd:  "baz",
			},
			{
				Base: "foo",
				Cmd:  "baz",
			},
		})),
	)
	require.NoError(t, err)

	t.Run("NoPattern", func(t *testing.T) {
		out := strings.Builder{}
		session.stdout = &strings.Builder{}

		expected := "bar\nbaz\nbaz\n"

		require.NoError(t, session.apps["history"].Run([]string{"!!history"}))
		require.Equal(t, expected, out.String())
	})

	t.Run("WithPattern", func(t *testing.T) {
		out := strings.Builder{}
		session.stdout = &strings.Builder{}

		expected := "bar\n"

		require.NoError(t, session.apps["history"].Run([]string{"!!history", "bar"}))
		require.Equal(t, expected, out.String())
	})

	t.Run("Show", func(t *testing.T) {
		out := strings.Builder{}
		session.stdout = &strings.Builder{}

		expected := "foo bar\nfoo baz\nfoo baz\n"

		require.NoError(t, session.apps["history"].Run([]string{"!!history", "--show"}))
		require.Equal(t, expected, out.String())
	})

	t.Run("N", func(t *testing.T) {
		out := strings.Builder{}
		session.stdout = &out

		expected := "baz\nbaz\n"

		require.NoError(t, session.apps["history"].Run([]string{"!!history", "--number", "2"}))
		require.Equal(t, expected, out.String())
	})
}

func TestExport(t *testing.T) {
	session, err := NewSession("foo",
		OptionInteractive(false),
	)
	require.NoError(t, err)

	t.Run("Ok", func(t *testing.T) {
		require.NoError(t, session.apps["export"].Run([]string{"!!export", "set", "foo=bar"}))
		assert.Equal(t, "bar", session.environ["foo"])
	})

	t.Run("BadArgs", func(t *testing.T) {
		session.environ = make(map[string]string)
		require.Error(t, session.apps["export"].Run([]string{"!!export", "foo=bar", "=baz"}))
		assert.Empty(t, session.environ["foo"])
	})

	t.Run("Show", func(t *testing.T) {
		out := strings.Builder{}
		session.stdout = &out

		session.environ["foo"] = "bar"
		session.environ["baz"] = ""

		expected := "foo='bar'\nbaz=''\n"
		require.NoError(t, session.apps["export"].Run([]string{"!!export", "show"}))
		require.Equal(t, expected, out.String())
	})
}
