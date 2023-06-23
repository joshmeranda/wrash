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
			Opt: Arg{
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
					Opt: Arg{
						Kind:    KindDefault,
						Choices: []string{"abc", "def"},
					},
				},
				"--bar": {
					Description: "takes a path value",
					Opt: Arg{
						Kind: KindPath,
					},
				},
				"--flag": {
					Description: "takes no value",
					Opt: Arg{
						Kind: KindNone,
					},
				},
			},
			Opt: Arg{
				Choices: []string{"first", "second", "third"},
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
		name         string
		args         []string
		completeLast bool
		expected     []prompt.Suggest
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
					Text: "first",
				},
				{
					Text: "second",
				},
				{
					Text: "third",
				},
			},
		},
		{
			name:         "WithSubCommandPrefix",
			args:         []string{"fo"},
			completeLast: true,
			expected: []prompt.Suggest{
				{
					Text:        "foo",
					Description: "foo subcommand",
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
			name: "FooWithFooAndArgs",
			args: []string{"foo", "--foo", "abc"},
			expected: []prompt.Suggest{
				{
					Text: "first",
				},
				{
					Text: "second",
				},
				{
					Text: "third",
				},
			},
		},
		{
			name: "FooWithFooAndArgs",
			args: []string{"foo", "--flag"},
			expected: []prompt.Suggest{
				{
					Text: "first",
				},
				{
					Text: "second",
				},
				{
					Text: "third",
				},
			},
		},
		{
			name:         "FooWithFooPrefix",
			args:         []string{"foo", "--f"},
			completeLast: true,
			expected: []prompt.Suggest{
				{
					Text:        "--flag",
					Description: "takes no value",
				},
				{
					Text:        "--foo",
					Description: "takes some value",
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
			actual := ExampleCommandSuggestion.Suggest(tc.args, tc.completeLast)
			sort.Slice(actual, func(i, j int) bool {
				return actual[i].Text < actual[j].Text
			})
			assert.Equal(t, tc.expected, actual)
		})
	}
}
