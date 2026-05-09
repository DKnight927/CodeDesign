---
[cd]
name = "audit"
version = "1.0.0"
product_kind = "audit"
platform = "responsive"

[cd.requires]
craft = ["anti-ai-slop"]

[cd.tool_augmentation]
critic_emit = """
Audit mode: prioritise `craft` and `functionality` dimensions. If a
finding's evidence cannot cite a concrete nodeId, omit it. No
speculative findings.
"""
---

# Audit skill

Critique-only runs against an existing Figma scene. Produces a
5-dim rubric report with actionable fixOps; does not emit a new
DesignPlan.
