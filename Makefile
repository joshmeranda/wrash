GO_BUILD:=go build -race
GO_FMT:=go fmt
GO_TEST:=go test
GO_GENERATE:=go generate

ifdef VERBOSE
        GO_BUILD += -v -x
        GO_TEST += -test.v
		GO_GENERATE += -v -x

        RM += --verbose
endif

TAG:=$(shell git tag --contains HEAD)

ifeq (${TAG},)
$(info no tag found for HEAD)
TAG:="$(shell git tag --sort version:refname --list | tail --lines 1)-$(shell git rev-parse HEAD)"
endif

ifneq ("$(shell git status --porcelain)",)
$(info HEAD is dirty)
TAG:=${TAG}-dirty
endif

$(info using tag ${TAG})

.PHONY: help

help:
	@echo "Available targets and values:"
	@echo "Targets:"
	@echo "  wrash 	     build wrash binary"
	@echo "  build       build all wrash artifacts"
	@echo "  clean       remove all wrash artifacts"
	@echo "  test        run tests"
	@echo ""
	@echo "Values:"
	@echo "  VERBOSE     run recipes with more verbose output"

# # # # # # # # # # # # # # # # # # # #
# Build recipes                       #
# # # # # # # # # # # # # # # # # # # #

.PHONY: build wrash

SOURCES=$(shell find . -name '*.go')

build: wrash

wrash: bin/wrash

bin/wrash: ${SOURCES}
	${GO_BUILD} -ldflags "-X main.Version=${TAG}" -o $@ .

# # # # # # # # # # # # # # # # # # # #
# Test recipes                        #
# # # # # # # # # # # # # # # # # # # #

# TEST_PKGS=./pkg 
TEST_PKGS=$(shell find . -name '*_test.go' -exec dirname '{}' + | sort | uniq)

test:
	${GO_TEST} ${TEST_PKGS}

# # # # # # # # # # # # # # # # # # # #
# Clean recipes                       #
# # # # # # # # # # # # # # # # # # # #

clean:
	${RM} --recursive bin