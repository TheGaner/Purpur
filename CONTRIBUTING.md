# Contributing to the standalone Rust server

**Mandatory review policy**: read docs/ROADMAP.md Phase -1 and docs/IMPLEMENTATION_REFERENCE_AUDIT.md before implementation. Every task must compare relevant upstream server implementations, document source paths/commits and alternatives, verify correctness and performance where applicable, and document the license boundary. Do not assume 48 passing tests prove parity/optimization.

For current project-wide retrospective hold:

    python3 scripts/reference_review_gate.py --check
    python3 scripts/reference_review_gate.py --enforce

--check validates the ledger structure and complete source inventory. --enforce additionally requires evidence-backed approvals and current source hashes; it intentionally fails while legacy modules remain pending. Never mark a review approved to get a green result without actual recorded, reproduced evidence.

Future new Rust source files MUST be mapped to an audit group; existing approved source modifications invalidate that approval until reviewed again. See docs/SOURCE_REVISIONS.md and docs/LICENSING.md for pinned references and provenance.
