package main

import (
	"os"
	"path"
	"path/filepath"
)

const (
	EnvHistoryFile = "WRASH_HISTORY_FILE"
	EnvConfigDir   = "WRASH_CONFIG_DIR"
	EnvLogFile     = "WRASH_LOG_FILE"
)

func GetHistoryFile() (string, error) {
	if path := os.Getenv(EnvHistoryFile); path != "" {
		return path, nil
	}

	dir, err := os.UserHomeDir()
	if err != nil {
		return "", err
	}

	return path.Join(dir, ".wrash_history.yaml"), nil
}

func GetConfigDir() (string, error) {
	if path := os.Getenv(EnvConfigDir); path != "" {
		return path, nil
	}

	dir, err := os.UserConfigDir()
	if err != nil {
		return "", err
	}

	return filepath.Join(dir, "wrash"), nil
}

func GetLogFile() (string, error) {
	if path := os.Getenv(EnvLogFile); path != "" {
		return path, nil
	}

	dir, err := os.UserHomeDir()
	if err != nil {
		return "", err
	}

	return filepath.Join(dir, ".wrash.log"), nil
}
