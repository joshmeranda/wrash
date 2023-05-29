GO_BUILD=go build -race
GO_FMT=go fmt
GO_TEST=go test

ifdef VERBOSE
        GO_BUILD += -v -x
        GO_TEST += -test.v

        RM += --verbose
endif

.PHONY: help

help:
	@echo "Available targets and values:"
	@echo "Targets:"
	@echo "  wrash 	     build wrash binary"
	@echo "  build       build all wrash artifacts"
	@echo ""
	@echo "Values:"
	@echo "  VERBOSE     run recipes with more verbose output"

# # # # # # # # # # # # # # # # # # # #
# Build recipes                       #
# # # # # # # # # # # # # # # # # # # #

.PHONY: build scrapedb

build: scrapedb

wrash: bin/wrash

bin/wrash:
	${GO_BUILD} -o $@ ./pkg/cmd/wrash


TEST_PKGS=./pkg 

test:
	${GO_TEST} ${TEST_PKGS}
