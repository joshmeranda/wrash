package args

import (
	"path"
	"testing"

	"github.com/stretchr/testify/assert"
)

var testDir = path.Join("..", "..", "tests", "resources", "a_directory")

func testEnv(name string) string {
	return map[string]string{
		"SOMETHING": "something",
		"SOME_VAR":  "some value",
	}[name]
}

func TestExpandWord(t *testing.T) {
	type testCase struct {
		Name string
		Node Node
		Out  string
	}

	cases := []testCase{
		// word
		{
			Name: "SimpleWord",
			Node: &Word{
				Value: "abc",
			},
			Out: "abc",
		},
		{
			Name: "WordWithWildcard",
			Node: &Word{
				Value: path.Join(testDir, "a*_file"),
			},
			Out: path.Join(testDir, "a_file") + " " + path.Join(testDir, "another_file"),
		},
		{
			Name: "WordWithEscapedWildcard",
			Node: &Word{
				Value: path.Join(testDir, "a\\*_file"),
			},
			Out: path.Join(testDir, "a*_file"),
		},

		// variable expansion
		{
			Name: "VariableExpansion",
			Node: &VariableExpansion{"SOME_VAR"},
			Out:  "some value",
		},
		{
			Name: "EmptyVariableExpansion",
			Node: &VariableExpansion{"NO_EXIST"},
			Out:  "",
		},

		// single qquote
		{
			Name: "SingleQuote",
			Node: &SingleQuote{"a'b'c"},
			Out:  "a'b'c",
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
			Out: "value of SOMETHING: something",
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
			Out: "*",
		},
	}

	for _, tc := range cases {
		t.Run(tc.Name, func(t *testing.T) {
			assert.Equal(t, tc.Out, tc.Node.Expand(testEnv))
		})
	}
}
