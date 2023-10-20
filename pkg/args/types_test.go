package args

import (
	"path"
	"testing"

	"github.com/stretchr/testify/assert"
	"github.com/stretchr/testify/require"
)

var testDir = path.Join("..", "..", "tests", "resources", "a_directory")

func testEnv(name string) string {
	return map[string]string{
		"SOMETHING": "something",
		"SOME_VAR":  "some value",
	}[name]
}

func TestNodeExpand(t *testing.T) {
	type testCase struct {
		Name       string
		Node       Node
		Out        []string
		ExpectsErr bool
	}

	cases := []testCase{
		// word
		{
			Name: "SimpleWord",
			Node: &Word{
				Value: "abc",
			},
			Out: []string{"abc"},
		},
		{
			Name: "WordWithWildcard",
			Node: &Word{
				Value: path.Join(testDir, "a*_file"),
			},
			Out: []string{path.Join(testDir, "a_file"), path.Join(testDir, "another_file")},
		},
		{
			Name: "WordWithEscapedWildcard",
			Node: &Word{
				Value: path.Join(testDir, "a\\*_file"),
			},
			Out: []string{path.Join(testDir, "a*_file")},
		},
		{
			Name: "WordWithWildcardNoMatches",
			Node: &Word{
				Value: "no_*_matches",
			},
			ExpectsErr: true,
		},

		// variable expansion
		{
			Name: "VariableExpansion",
			Node: &VariableExpansion{"SOME_VAR"},
			Out:  []string{"some value"},
		},
		{
			Name: "EmptyVariableExpansion",
			Node: &VariableExpansion{"NO_EXIST"},
			Out:  []string{""},
		},

		// single qquote
		{
			Name: "SingleQuote",
			Node: &SingleQuote{"a'b'c"},
			Out:  []string{"a'b'c"},
		},

		// double quote
		{
			Name: "DoubleQuote",
			Node: &DoubleQuote{
				Nodes: []Node{
					&Word{
						Value:    "value of SOMETHING: ",
						IsQuoted: true,
					},
					&VariableExpansion{"SOMETHING"},
				},
			},
			Out: []string{"value of SOMETHING: something"},
		},
		{
			Name: "DoubleQuoteWithQildcard",
			Node: &DoubleQuote{
				Nodes: []Node{
					&Word{
						Value:    "*",
						IsQuoted: true,
					},
				},
			},
			Out: []string{"*"},
		},

		// failing expansions
		{
			Name: "UnterminatedBraceExpansion",
			Node: &Word{
				Value: "abcdefg[]",
			},
			Out:        nil,
			ExpectsErr: true,
		},
	}

	for _, tc := range cases {
		t.Run(tc.Name, func(t *testing.T) {
			actual, err := tc.Node.Expand(testEnv)
			if !tc.ExpectsErr {
				assert.NoError(t, err)
			}
			assert.Equal(t, tc.Out, actual)
		})
	}
}

func TestCommandArgs(t *testing.T) {
	cmd, err := Parse("i 'want'   $NUM  \"$ITEM's\"")
	require.NoError(t, err)
	assert.Equal(t, []string{"i", "'want'", "$NUM", "\"$ITEM's\""}, cmd.Args())
}
