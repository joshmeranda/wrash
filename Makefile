TAG:=$(shell git tag --contains HEAD)

ifeq (${TAG},)
$(info no tag found for HEAD)
TAG:=$(shell git tag --sort version:refname --list | tail --lines 1)-$(shell git rev-parse HEAD)
endif

ifneq ($(shell git status --porcelain),)
$(info HEAD is dirty)
TAG:=${TAG}-dirty
endif

$(info using tag ${TAG})

BUILD_FLAGS=-ldflags "-X main.Version=${TAG}"

GO_BUILD=go build ${BUILD_FLAGS}
GO_INSTALL=go install ${BUILD_FLAGS}
GO_FMT=go fmt
GO_TEST=go test

ifdef VERBOSE
        GO_BUILD += -v -x
        GO_INSTALL += -v -x
        GO_TEST += -test.v

        RM += --verbose
endif

.PHONY: help

help:
	@echo "Available targets and values:"
	@echo "Targets:"
	@echo "  build       build all wrash artifacts"
	@echo "  install    build and install wrash"
	@echo "  clean       remove all wrash artifacts"
	@echo "  test        run tests"
	@echo ""
	@echo "Values:"
	@echo "  VERBOSE     run recipes with more verbose output"

# todo: add install to v0.4.0

# # # # # # # # # # # # # # # # # # # #
# Build recipes                       #
# # # # # # # # # # # # # # # # # # # #

.PHONY: build

SOURCES=$(shell find . -name '*.go')

build: wrash

wrash: bin/wrash

bin/wrash: ${SOURCES}
	${GO_BUILD} -o $@ .

# # # # # # # # # # # # # # # # # # # #
# Install recipes                     #
# # # # # # # # # # # # # # # # # # # #

.PHONY: install

install:
	${GO_INSTALL} .

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