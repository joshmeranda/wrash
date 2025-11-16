package wrash

import (
	"fmt"
	"os"
	"path/filepath"
	"testing"

	prompt "github.com/joshmeranda/go-prompt"
	"github.com/stretchr/testify/assert"
	"github.com/stretchr/testify/require"
)

// writeScript writes a new script in the provided directory.
func writeScript(dir string, content string) (string, error) {
	f, err := os.CreateTemp(dir, "")
	if err != nil {
		return "", err
	}

	if err := f.Chmod(0o755); err != nil {
		return "", err
	}

	if _, err = f.Write([]byte(content)); err != nil {
		return "", err
	}

	if err = f.Close(); err != nil {
		return "", err
	}

	return f.Name(), nil
}

func TestFileCompleter(t *testing.T) {
	cases := []struct {
		name string
		line string

		expected []prompt.Suggest
	}{
		{
			name: "empty line",

			expected: []prompt.Suggest{},
		},
		{
			name: "with prefix",
			line: "h",

			expected: []prompt.Suggest{
				{
					Text: "history.go",
				},
				{
					Text: "history_test.go",
				},
			},
		},
		{
			name: "with wildcard",
			line: "*_test.go",

			expected: []prompt.Suggest{
				{
					Text: "builtins_test.go",
				},
				{
					Text: "complete_test.go",
				},
				{
					Text: "history_test.go",
				},
				{
					Text: "session_test.go",
				},
			},
		},
		{
			name: "with current dir",
			line: "./o",

			expected: []prompt.Suggest{
				{
					// todo: this should really be "./options.go"
					Text: "options.go",
				},
			},
		},
		{
			name: "with external dir",
			line: "../tests/resources/",

			expected: []prompt.Suggest{
				{
					Text: "../tests/resources/a_directory/",
				},
				{
					Text: "../tests/resources/history/",
				},
				{
					Text: "../tests/resources/some_other_directory/",
				},
			},
		},
	}

	for _, c := range cases {
		t.Run(c.name, func(t *testing.T) {
			buff := prompt.NewBuffer()
			buff.InsertText(c.line, false, true)

			actual := fileCompleter(*buff.Document())

			assert.Equal(t, c.expected, actual)
		})
	}
}

func TestBuiltinCompleter(t *testing.T) {
	session, err := NewSession("git", OptionInteractive(false))
	require.NoError(t, err)

	cases := []struct {
		name string
		line string

		expected []prompt.Suggest
	}{
		{
			name: "no prefix",
			line: "!!",

			expected: []prompt.Suggest{
				{
					Text:        "!!cd",
					Description: "change the working directory of the shell",
				},
				{
					Text:        "!!complete",
					Description: "configure command completion",
				},
				{
					Text:        "!!env",
					Description: "set or display environment variables for the current session",
				},
				{
					Text:        "!!exit",
					Description: "exit the shell",
				},
				{
					Text:        "!!help",
					Description: "view help text",
				},
				{
					Text:        "!!history",
					Description: "view the history of the shell (pattern should not include the base command)",
				},
			},
		},
		{
			name: "with prefix",
			line: "!!h",

			expected: []prompt.Suggest{
				{
					Text:        "!!help",
					Description: "view help text",
				},
				{
					Text:        "!!history",
					Description: "view the history of the shell (pattern should not include the base command)",
				},
			},
		},
		{
			name: "with no match",
			line: "!!abc",

			expected: []prompt.Suggest{},
		},
	}

	for _, c := range cases {
		t.Run(c.name, func(t *testing.T) {
			buff := prompt.NewBuffer()
			buff.InsertText(c.line, false, true)

			actual := session.builtinsCompleter(*buff.Document())

			assert.Equal(t, c.expected, actual)
		})
	}
}

func TestCommandCompleter(t *testing.T) {
	tmpDir := t.TempDir()
	scriptPathWithSuggestions := fmt.Sprintf("%s%s%s", tmpDir, string(os.PathSeparator), "complete.sh")

	// todo: this might causes problems in other tests which rely on Path
	pathBackup := os.Getenv("PATH")
	os.Setenv("PATH", fmt.Sprintf("%s:%s", pathBackup, tmpDir))
	t.Cleanup(func() {
		os.Setenv("PATH", pathBackup)
	})

	scriptPathWithSuggestions, err := writeScript(tmpDir,
		`#!/usr/bin/sh
		echo option 1
		echo option 2
		echo last option`,
	)
	require.NoError(t, err)

	t.Run("new completion", func(t *testing.T) {
		cases := []struct {
			name string

			disableFile bool
			path        string

			expected commandCompletion
			err      string
		}{
			{
				name: "absolute path exists",
				path: scriptPathWithSuggestions,

				expected: commandCompletion{
					path: scriptPathWithSuggestions,
				},
			},
			{
				// todo: we probably want to update this to something more controllable (add scriptPath to PATH and run)
				name: "path in PATH",
				path: filepath.Base(scriptPathWithSuggestions),

				expected: commandCompletion{
					path: scriptPathWithSuggestions,
				},
			},
			{
				name: "path does not exist",
				path: "i.do.not.exist",

				err: "exec: \"i.do.not.exist\": executable file not found in $PATH",
			},
		}

		for _, c := range cases {
			t.Run(c.name, func(t *testing.T) {
				completer, err := NewCompletion(c.disableFile, c.path)

				if c.err == "" {
					assert.NoError(t, err)
					assert.Equal(t, c.expected, *(completer.(*commandCompletion)))
				} else {
					assert.Nil(t, completer)
					assert.EqualError(t, err, c.err)
				}
			})
		}
	})

	t.Run("test complete", func(t *testing.T) {
		cases := []struct {
			name      string
			line      string
			completer Completer

			expected []prompt.Suggest
		}{
			{
				name: "script with completion",
				completer: &commandCompletion{
					path: scriptPathWithSuggestions,
				},

				expected: []prompt.Suggest{
					{
						Text: "option 1",
					},
					{
						Text: "option 2",
					},
					{
						Text: "last option",
					},
				},
			},
			{
				name: "script with completion with prefix",
				line: "option",
				completer: &commandCompletion{
					path: scriptPathWithSuggestions,
				},

				expected: []prompt.Suggest{
					{
						Text: "option 1",
					},
					{
						Text: "option 2",
					},
				},
			},
			{
				name: "script without completions without file backup",
				completer: &commandCompletion{
					disableFile: true,
					path:        "echo",
				},

				expected: []prompt.Suggest{},
			},
			{
				name: "script without completions with file backup",
				line: scriptPathWithSuggestions[:len(scriptPathWithSuggestions)-1],
				completer: &commandCompletion{
					path: "echo",
				},

				expected: []prompt.Suggest{
					{
						Text: scriptPathWithSuggestions,
					},
				},
			},
		}

		for _, c := range cases {
			t.Run(c.name, func(t *testing.T) {
				buff := prompt.NewBuffer()
				buff.InsertText(c.line, false, true)

				actual := c.completer.Complete(*buff.Document())

				assert.Equal(t, c.expected, actual)
			})
		}
	})
}
