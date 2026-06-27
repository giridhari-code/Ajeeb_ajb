# Ajeeb — Root Makefile (delegates to ajeebc/)
# See ajeebc/Makefile for all build/test/bootstrap targets.

.PHONY: all native rust test bootstrap bootstrap-full clean clean-all help

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

help:
	cd ajeebc && $(MAKE) help
