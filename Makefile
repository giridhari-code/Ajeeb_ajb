# Ajeeb — root Makefile (delegates to ajeebc/)

.PHONY: all native rust test bootstrap bootstrap-full clean clean-all clean-cache help

all: native

native:
	cd ajeebc && $(MAKE) native

rust:
	cd ajeebc && $(MAKE) rust

test:
	cd ajeebc && $(MAKE) test

bootstrap:
	cd ajeebc && $(MAKE) bootstrap

bootstrap-full:
	cd ajeebc && $(MAKE) bootstrap-full

clean:
	cd ajeebc && $(MAKE) clean

clean-all:
	cd ajeebc && $(MAKE) clean-all

clean-cache:
	rm -rf .ajeeb_cache ajeebc/.ajeeb_cache
	@echo "Cache cleared."

help:
	cd ajeebc && $(MAKE) help
