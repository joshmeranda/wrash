package main

import (
	"fmt"
	"testing"

	"github.com/stretchr/testify/assert"
)

func TestSplitEnviorn(t *testing.T) {
	type Result struct {
		Key   string
		Value string
		Err   error
	}

	type TestCase struct {
		Name     string
		Input    string
		Expected Result
	}

	testCases := []TestCase{
		{
			Name:  "Simple",
			Input: "aA0_=some value",
			Expected: Result{
				Key:   "aA0_",
				Value: "some value",
				Err:   nil,
			},
		},
		{
			Name:  "EmptyValue",
			Input: "aA0_=",
			Expected: Result{
				Key:   "aA0_",
				Value: "",
				Err:   nil,
			},
		},
		{
			Name:  "NoEqual",
			Input: "aA0_",
			Expected: Result{
				Key:   "",
				Value: "",
				Err:   fmt.Errorf("no '=' found in environment variable 'aA0_'"),
			},
		},
		{
			Name:  "InvalidName",
			Input: "invalid-name=100",
			Expected: Result{
				Key:   "",
				Value: "",
				Err:   fmt.Errorf("invalid identifier 'invalid-name', must match pattern ^[a-zA-Z0-9_]+$"),
			},
		},
	}

	for _, tc := range testCases {
		t.Run(tc.Name, func(t *testing.T) {
			key, value, err := splitEnviron(tc.Input)
			result := Result{
				Key:   key,
				Value: value,
				Err:   err,
			}

			assert.Equal(t, tc.Expected, result)
		})
	}
}
