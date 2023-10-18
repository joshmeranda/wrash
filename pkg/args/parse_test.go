package args

import (
	"testing"

	"github.com/r3labs/diff/v3"
	"github.com/stretchr/testify/assert"
)

func TestParse(t *testing.T) {
	type result struct {
		Cmd Command
		Err error
	}

	type testCase struct {
		Name   string
		Input  string
		Result result
	}

	testCases := []testCase{
		{
			Name:  "Empty",
			Input: "",
			Result: result{
				Cmd: Command{},
			},
		},
		{
			Name:  "Simple",
			Input: "abc '$SOME_VAR \\'' \"$SOME_VAR  d\\\"e\\\"f\"  $SOMETHING    g'h'j",
			Result: result{
				Cmd: Command{
					Arg{
						&Word{
							Value: "abc",
						},
					},
					Arg{
						&SingleQuote{
							Value: "$SOME_VAR '",
						},
					},
					Arg{
						&DoubleQuote{
							Nodes: []Node{
								&VariableExpansion{
									Name: "SOME_VAR",
								},
								&Word{
									Value:    "  d\"e\"f",
									IsQuoted: true,
								},
							},
						},
					},
					Arg{
						&VariableExpansion{
							Name: "SOMETHING",
						},
					},
					Arg{
						&Word{
							Value: "g",
						},
						&SingleQuote{
							Value: "h",
						},
						&Word{
							Value: "j",
						},
					},
				},
			},
		},
		{
			Name:  "UnterminatedSingleQuote",
			Input: "'abc",
			Result: result{
				Err: ErrUnexpectedEOF{
					Cause: ErrUnterminatedSequence{
						Start: "'",
						End:   "'",
					},
				},
			},
		},
		{
			Name:  "UnterminatedDoubleQuote",
			Input: "\"abc",
			Result: result{
				Err: ErrUnexpectedEOF{
					Cause: ErrUnterminatedSequence{
						Start: "\"",
						End:   "\"",
					},
				},
			},
		},
	}

	for _, tc := range testCases {
		t.Run(tc.Name, func(t *testing.T) {
			cmd, err := Parse(tc.Input)
			assert.Equal(t, tc.Result.Err, err)

			changelog, err := diff.Diff(tc.Result.Cmd, cmd)
			assert.NoError(t, err)
			assert.Empty(t, changelog)
		})
	}
}
