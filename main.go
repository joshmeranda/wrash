package main

import (
	"fmt"
	"log/slog"
	"os"
	"os/exec"
	"strings"

	wrash "github.com/joshmeranda/wrash/pkg"
	"github.com/joshmeranda/wrash/pkg/args"
	"github.com/joshmeranda/wrash/pkg/log"
	"github.com/urfave/cli/v2"
)

var Version string = ""

func setup(ctx *cli.Context) error {
	logFile, err := GetLogFile()
	if err != nil {
		return fmt.Errorf("could not determine log file: %w", err)
	}

	handler, err := log.NewFileHandler(logFile, &log.Options{
		Level: slog.LevelInfo,
	})
	if err != nil {
		return err
	}

	log.SetLogger(slog.New(handler))

	return nil
}

func run(ctx *cli.Context) error {
	env := loadEnviron(nil)

	rawBase := strings.Join(ctx.Args().Slice(), " ")
	if rawBase == "" {
		return fmt.Errorf("no command provided")
	}

	base, err := args.Parse(rawBase)
	if err != nil {
		return fmt.Errorf("could not parse command args: %w", err)
	}

	expanded, err := base.Expand(func(s string) string {
		return env[s]
	})
	if err != nil {
		return fmt.Errorf("could not expand args: %s", err)
	}

	if _, err := exec.LookPath(expanded[0]); err != nil {
		return fmt.Errorf("command not found: %s", expanded[0])
	}

	historyPath, err := GetHistoryFile()
	if err != nil {
		return fmt.Errorf("could not determine history file: %w", err)
	}

	configDir, err := GetConfigDir()
	if err != nil {
		return fmt.Errorf("cocould not determine config dir: %w", err)
	}

	entries, err := loadHistoryEntries(historyPath)
	if err != nil {
		return err
	}

	history := wrash.NewHistory(rawBase, wrash.NewHistoryWriter(historyPath), entries)

	session, err := wrash.NewSession(rawBase,
		wrash.OptionHistory(history),
		wrash.OptionEnvironment(env),
		wrash.OptionConfigDir(configDir),
	)
	if err != nil {
		return err
	}

	session.Run()

	return nil
}

func main() {
	app := &cli.App{
		Name:        "wrash",
		Version:     Version,
		Description: "turn wrap any command line utility into an interactive shell",
		Flags:       []cli.Flag{},
		Before:      setup,
		Action:      run,
		Authors: []*cli.Author{
			{
				Name:  "Josh Meranda",
				Email: "joshmeranda@gmail.com",
			},
		},
	}

	if err := app.Run(os.Args); err != nil {
		fmt.Fprintf(os.Stderr, "Error: %s", err)
	}
}
