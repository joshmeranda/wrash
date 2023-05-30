package wrash

import (
	"os"
	"testing"

	prompt "github.com/joshmeranda/go-prompt"
	"github.com/stretchr/testify/assert"
	"github.com/stretchr/testify/require"
)

func TestFileComplete(t *testing.T) {
	old, err := os.Getwd()
	require.NoError(t, err)

	require.NoError(t, os.Chdir("../tests"))
	defer os.Chdir(old)

	testCases := []struct {
		Name     string
		Text     string
		Expected []prompt.Suggest
	}{
		{"Empty", "", []prompt.Suggest{}},
		{"WithDot", "./", []prompt.Suggest{
			{Text: "resources/"},
		}},
		{"Prefix", "res", []prompt.Suggest{
			{Text: "resources/"},
		}},
		{"NoMatch", "nomoatch", []prompt.Suggest{}},

		{"SubDirMatch", "resources/a_directory/", []prompt.Suggest{
			{Text: "resources/a_directory/a_file"},
			{Text: "resources/a_directory/another_file"},
			{Text: "resources/a_directory/directory/"},
			{Text: "resources/a_directory/some_other_file"},
		}},
	}

	for _, tc := range testCases {
		t.Run(tc.Name, func(t *testing.T) {
			actual := getFilesWithPrefix(tc.Text)
			assert.Equal(t, tc.Expected, actual)
		})
	}
}
