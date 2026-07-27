---
lang: en
slug: versioning
title: What is versioned
description: A practical boundary between canonical project knowledge, derived caches, and machine-local traces.
section: Verify
order: 7
---

## Commit these

Version `.rationale/` with the project: Subjects, Records, evidence, proposals
until reviewed, schemas, configuration, and lifecycle history. These files
are the shareable explanation of why code must behave a certain way.

## Keep these local

`.rationale-local/` contains logs and local coordination state. SQLite/FTS is a
derived cache and can be rebuilt. The installed binary and agent manifests are
environment artifacts unless your team deliberately versions them.

## Release truth

The public preview currently documented here is `v0.1.0-alpha.4`. Changes in
the working tree remain Unreleased until a release is cut and its checksums,
installers, tests, and human review gates are complete.

## Safe recovery

Deleting a derived cache does not delete authority. Deleting a canonical Record
can remove historical context and must happen through Git review and the
documented lifecycle, not a cleanup script.
