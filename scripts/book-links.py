#!/usr/bin/env python3
"""mdbook preprocessor (docs/book.toml): the guide's links that leave the book.

The guide's pages link to the rest of the repository with relative links
(`../GLOSSARY.md`, `../../examples/…`), which work on GitHub and for the
authors' agent skill but point nowhere on the site. This rewrites them:

- `../GLOSSARY.md` becomes the book's own glossary page (`glossary.md`,
  which includes it);
- any other link out of `docs/guide/` becomes a GitHub link at `BOOK_REF`
  (default `main`; the release deploy sets the tag), under the book's
  `output.html.git-repository-url`.

Standard library only; mdbook runs it from the book's root (`docs/`).
"""

import json
import os
import posixpath
import re
import sys

SRC = "docs/guide"  # The book's `src`, from the repository root.
REWRITES = {"docs/GLOSSARY.md": f"{SRC}/glossary.md"}

# `](../target)` inline links, and `[label]: ../target` definitions.
INLINE = re.compile(r"(\]\()(\.\./[^)\s]+)(\))")
DEFINITION = re.compile(r"^(\s*\[[^\]]+\]:\s*)(\.\./\S+)()", re.MULTILINE)


def rewrite(content, chapter_dir, repo_url, ref):
    def link(match):
        target, _, fragment = match.group(2).partition("#")
        fragment = f"#{fragment}" if fragment else ""
        path = posixpath.normpath(posixpath.join(SRC, chapter_dir, target))
        path = REWRITES.get(path, path)
        if path.startswith(f"{SRC}/"):
            new = posixpath.relpath(path, posixpath.join(SRC, chapter_dir))
        else:
            new = f"{repo_url}/blob/{ref}/{path}"
        return f"{match.group(1)}{new}{fragment}{match.group(3)}"

    return DEFINITION.sub(link, INLINE.sub(link, content))


def chapters(items):
    for item in items:
        if isinstance(item, dict) and "Chapter" in item:
            yield item["Chapter"]
            yield from chapters(item["Chapter"]["sub_items"])


def main():
    if len(sys.argv) > 1:  # `supports <renderer>`: every renderer.
        return
    context, book = json.load(sys.stdin)
    repo_url = context["config"]["output"]["html"]["git-repository-url"].rstrip("/")
    ref = os.environ.get("BOOK_REF") or "main"
    for chapter in chapters(book["items"]):
        source = chapter.get("source_path")
        if source:
            chapter_dir = posixpath.dirname(source)
            chapter["content"] = rewrite(chapter["content"], chapter_dir, repo_url, ref)
    json.dump(book, sys.stdout)


if __name__ == "__main__":
    main()
