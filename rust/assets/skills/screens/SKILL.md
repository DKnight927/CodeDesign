---
[cd]
name = "screens"
version = "1.0.0"
product_kind = "screens"
platform = "responsive"

[cd.requires]
tokens = ["color.brand.primary"]
craft = ["anti-ai-slop", "accessibility-baseline"]

[cd.tool_augmentation]
plan_emit = """
When emitting screens:
- Each frame MUST have a clear primary action within the first 3 vertical units.
- Empty states MUST resolve a referenced illustration token or deliberately omit imagery.
- Do not emit more than 6 frames per plan; split into step plans if larger.
"""
---

# Screens skill

Generate multi-screen product flows from a PRD. Every frame carries
explicit empty / error / loading states unless the Brief opts out.
