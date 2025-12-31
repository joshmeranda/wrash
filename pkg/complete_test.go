package wrash

import (
	"os"
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
					Text: "./options.go",
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

	scriptPathWithSuggestions, err := writeScript(tmpDir,
		`#!/usr/bin/sh
		echo option 1
		echo option 2
		echo last option`,
	)
	require.NoError(t, err)

	cases := []struct {
		name      string
		line      string
		completer Completer

		expected []prompt.Suggest
	}{
		{
			name: "with empty line",
			completer: &commandCompleter{
				command: []string{"printf", "abc\ndef"},
			},

			expected: []prompt.Suggest{},
		},
		{
			name: "with command with suggestions with prefix",
			line: "a",
			completer: &commandCompleter{
				disableFile: true,
				command:     []string{"printf", "abc\ndef"},
			},

			expected: []prompt.Suggest{
				{
					Text: "abc",
				},
			},
		},
		{
			name: "with path without completions without file backup",
			completer: &commandCompleter{
				disableFile: true,
				command:     []string{"echo"},
			},

			expected: []prompt.Suggest{},
		},
		{
			name: "with path without completions with file backup",
			line: scriptPathWithSuggestions[:len(scriptPathWithSuggestions)-1],
			completer: &commandCompleter{
				command: []string{"echo"},
			},

			expected: []prompt.Suggest{
				{
					Text: scriptPathWithSuggestions,
				},
			},
		},
		{
			name: "with subcommands",
			line: "a",
			completer: &commandCompleter{
				disableFile: true,
				subcommands: map[string]*commandCompleter{
					"add":    {disableFile: true},
					"access": {disableFile: true},
					"remove": {disableFile: true},
				},
			},

			expected: []prompt.Suggest{
				{
					Text: "access",
				},
				{
					Text: "add",
				},
			},
		},
		{
			name: "with subcommand completer",
			line: "add opt",
			completer: &commandCompleter{
				disableFile: true,
				subcommands: map[string]*commandCompleter{
					"add": {
						disableFile: true,
						command:     []string{scriptPathWithSuggestions},
					},
				},
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
			name: "with subcommand with empty line",
			line: "",
			completer: &commandCompleter{
				subcommands: map[string]*commandCompleter{
					"bar": {},
				},
			},
			expected: []prompt.Suggest{},
		},
		{
			name: "with parse error",
			line: "echo 'unterminated single-quote",
			completer: &commandCompleter{
				command: []string{scriptPathWithSuggestions},
			},

			expected: make([]prompt.Suggest, 0),
		},

		{
			// Verify that the current status of the wrash prompt is passed to command completers via environment variable.
			name: "prompt passed to command via env",
			line: "abc def 'ghi' ",
			completer: &commandCompleter{
				command: []string{"bash", "-c", "echo $" + EnvCompleteCurrentPrompt},
			},

			expected: []prompt.Suggest{
				{
					Text: "abc def 'ghi'",
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
}
