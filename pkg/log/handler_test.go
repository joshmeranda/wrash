package log

import (
	"context"
	"fmt"
	"log/slog"
	"os"
	"path/filepath"
	"testing"

	"github.com/stretchr/testify/assert"
	"github.com/stretchr/testify/require"
)

func TestFileHandler(t *testing.T) {
	testDir := t.TempDir()

	defaultFactory := func(t *testing.T) string {
		t.Helper()

		f, err := os.CreateTemp(testDir, "")
		require.NoError(t, err)

		return f.Name()
	}

	t.Run("NewFileHandler", func(t *testing.T) {
		cases := []struct {
			name        string
			fileFactory func(t *testing.T) string

			shouldExist bool
			errMsg      string
		}{
			{
				name: "with logFile no existing",
				fileFactory: func(t *testing.T) string {
					return filepath.Join(testDir, "")
				},

				shouldExist: true,
			},
			{
				name: "with existing file",

				shouldExist: true,
			},
			{
				name: "with parent directories do not exist",
				fileFactory: func(t *testing.T) string {
					return filepath.Join(testDir, "i.do.no.exist", "some.file")
				},

				errMsg: fmt.Sprintf("failed to open log file: open %s: no such file or directory", filepath.Join(testDir, "i.do.no.exist", "some.file")),
			},
		}

		for _, c := range cases {
			t.Run(c.name, func(t *testing.T) {
				if c.name != "with parent directories do not exist" {
					return
				}

				fileFactory := defaultFactory
				if c.fileFactory != nil {
					fileFactory = c.fileFactory
				}

				logPath := fileFactory(t)

				_, err := NewFileHandler(logPath, &Options{})

				if c.shouldExist {
					assert.FileExists(t, logPath)
				} else {
					assert.NoFileExists(t, logPath)
				}

				if c.errMsg != "" {
					assert.EqualError(t, err, c.errMsg)
				} else {
					assert.NoError(t, err)
				}
			})
		}
	})

	t.Run("Enabled", func(t *testing.T) {
		cases := []struct {
			name string

			level slog.Level

			expected bool
		}{
			{
				name: "with greater level",

				level:    slog.LevelError,
				expected: true,
			},
			{
				name: "with equal level",

				level:    slog.LevelError,
				expected: true,
			},
			{
				name: "with lesser level",

				level:    slog.LevelError,
				expected: true,
			},
		}

		handler := fileHandler{
			level: slog.LevelInfo,
		}

		for _, c := range cases {
			t.Run(c.name, func(t *testing.T) {
				actual := handler.Enabled(context.Background(), c.level)
				assert.Equal(t, c.expected, actual)
			})
		}
	})
}
