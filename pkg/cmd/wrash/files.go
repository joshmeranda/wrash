package main

import (
	"fmt"
	"os"
	"path"
)

const (
	EnvCompletionDir = "WRASH_COMPLETION_DIR"
	EnvHistoryFile   = "WRASH_HISTORY_FILE"
)

// todo: add enviornment variables for this

func GetHistoryFile() (string, error) {
	if path := os.Getenv(EnvHistoryFile); path != "" {
		return path, nil
	}

	dir, err := os.UserHomeDir()
	if err != nil {
		return "", fmt.Errorf("could not determine user home directory: %w", err)
	}

	return path.Join(dir, ".wrash_history.yaml"), nil
}

func getCompletionDir() (string, error) {
	if path := os.Getenv(EnvCompletionDir); path != "" {
		return path, nil
	}

	dir, err := os.UserHomeDir()
	if err != nil {
		return "", fmt.Errorf("could not determine user home directory: %w", err)
	}

	return path.Join(dir, ".wrash_completion.yaml"), nil
}

func GetCompletionFile(base string) (string, error) {
	dir, err := getCompletionDir()
	if err != nil {
		return "", err
	}

	return path.Join(dir, base), nil
}
