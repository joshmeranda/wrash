package wrash

import (
	"os"
	"sort"
	"testing"

	prompt "github.com/joshmeranda/go-prompt"
	"github.com/stretchr/testify/assert"
	"github.com/stretchr/testify/require"
)

var ExampleCommandSuggestion = &CommandSuggestion{
	Description: "an example suggestion for wrash",
	Flags: map[string]FlagSuggestion{
		"--help": {
			Description: "show help for example",
			Opts: Arg{
				Kind: KindNone,
			},
		},
	},
	SubCommands: map[string]CommandSuggestion{
		"foo": {
			Description: "foo subcommand",
			Flags: map[string]FlagSuggestion{
				"--foo": {
					Description: "takes some value",
					Opts: Arg{
						Kind:    KindDefault,
						Choices: []string{"abc", "def"},
					},
				},
				"--bar": {
					Description: "takes a path value",
					Opts: Arg{
						Kind: KindPath,
					},
				},
			},
		},
	},
}

func TestLoad(t *testing.T) {
	suggestion, err := LoadSuggestions("../examples/completion_example.yaml")
	assert.NoError(t, err)
	assert.Equal(t, ExampleCommandSuggestion, suggestion)
}

func TestSuggest(t *testing.T) {
	oldDir, err := os.Getwd()
	require.NoError(t, err)
	require.NoError(t, os.Chdir("../tests/resources"))

	defer os.Chdir(oldDir)

	type testCase struct {
		name     string
		args     []string
		expected []prompt.Suggest
	}

	testCases := []testCase{
		{
			name: "empty",
			args: []string{},
			expected: []prompt.Suggest{
				{
					Text:        "foo",
					Description: "foo subcommand",
				},
			},
		},
		{
			name: "WithSubCommand",
			args: []string{"foo"},
			expected: []prompt.Suggest{
				{
					Text:        "--bar",
					Description: "takes a path value",
				},
				{
					Text:        "--foo",
					Description: "takes some value",
				},
			},
		},
		{
			name: "FooWithFoo",
			args: []string{"foo", "--foo"},
			expected: []prompt.Suggest{
				{
					Text: "abc",
				},
				{
					Text: "def",
				},
			},
		},
		{
			name: "FooWithBar",
			args: []string{"foo", "--bar"},
			expected: []prompt.Suggest{
				{
					Text: "a_directory",
				},
				{
					Text: "history",
				},
				{
					Text: "some_other_directory",
				},
			},
		},
	}

	for _, tc := range testCases {
		t.Run(tc.name, func(t *testing.T) {
			actual := ExampleCommandSuggestion.Suggest(tc.args)
			sort.Slice(actual, func(i, j int) bool {
				return actual[i].Text < actual[j].Text
			})
			assert.Equal(t, tc.expected, actual)
		})
	}
}
