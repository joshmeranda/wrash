package wrash

import (
	"testing"

	"github.com/stretchr/testify/assert"
)

func TestGetNextBoundary(t *testing.T) {
	runeset := "-_"
	testCases := []struct {
		text     string
		runeset  string
		expected int
	}{
		{"", runeset, 0},
		{"a", runeset, 1},
		{"ab c", runeset, 2},
		{"a-b", runeset, 1},
		{"a_b", runeset, 1},
		{"a  b", runeset, 1},
		{"-_-", runeset, 3},
	}

	for _, tc := range testCases {
		t.Run("TestGetNextBoundary"+tc.text, func(t *testing.T) {
			i := getNextBoundary(tc.runeset, tc.text)
			assert.Equal(t, tc.expected, i, "getNextBoundary(\"%s\", \"%s\") should return %d", tc.runeset, tc.text, tc.expected)
		})
	}
}
