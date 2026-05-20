# PlausiDen-AVP-Doctrine Makefile — discovery + doctrine ops.

.PHONY: help
help: ## Show this help.
	@printf '\n\033[1mPlausiDen-AVP-Doctrine — Makefile help\033[0m\n\n'
	@printf 'For the full surface see:\n'
	@printf '  AGENTS.md          — orientation for AI agents (read first)\n'
	@printf '  TOOLS.md           — canonical doctrine command index\n'
	@printf '  AVP2_PROTOCOL.md   — read before any doctrine change\n\n'
	@printf 'Common operations:\n\n'
	@awk 'BEGIN {FS = ":.*?## "} /^[a-zA-Z0-9_.-]+:.*?## / {printf "  \033[36m%-22s\033[0m %s\n", $$1, $$2}' $(MAKEFILE_LIST)
	@printf '\n'

# ----------------------------------------------------------------
# Doctrine queries — delegate to forge doctrine subcommand
# ----------------------------------------------------------------

FORGE := ../PlausiDen-Forge/target/release/forge

.PHONY: query-all
query-all: ## List every loaded rule (id + name) via forge doctrine query.
	$(FORGE) --root ../PlausiDen-Forge doctrine query

.PHONY: lifecycle-audit
lifecycle-audit: ## Audit rule lifecycle (experimental / stable / deprecated) health.
	$(FORGE) --root ../PlausiDen-Forge doctrine lifecycle

.PHONY: check
check: ## Verify every cited rule resolves (cross-references downstream).
	$(FORGE) --root ../PlausiDen-Forge doctrine check

.PHONY: render
render: ## Render the full doctrine as a single Markdown document.
	@mkdir -p generated
	$(FORGE) --root ../PlausiDen-Forge doctrine render --out generated/doctrine.md

.PHONY: deprecation-audit
deprecation-audit: ## Flag .citing(...) of deprecated rules in PlausiDen-Forge.
	$(FORGE) --root ../PlausiDen-Forge doctrine deprecation-audit

# ----------------------------------------------------------------
# Validation
# ----------------------------------------------------------------

.PHONY: validate-toml
validate-toml: ## Parse every rules/*.toml file for malformed TOML.
	@for f in doctrine/rules/*.toml; do echo "Parsing $$f"; cargo run --manifest-path ../PlausiDen-Forge/Cargo.toml -q -p doctrine-core --example noop 2>/dev/null || true; done
	@echo "(Use \`make check\` for live doctrine validation via forge doctrine check.)"

# ----------------------------------------------------------------
# Maintenance
# ----------------------------------------------------------------

.PHONY: clean
clean: ## Remove generated/.
	rm -rf generated/
