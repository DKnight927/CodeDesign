// codedesign runtime shim v0.0.1
// -----------------------------------------------------------------------------
// This block is prepended to every compiled plugin script so the generated
// code has something to call. It hard-codes the default DS (default@1.0.0)
// into concrete Figma primitives: colors -> RGB, typography -> fontSize + Inter,
// spacing -> numeric px. Components fall back to labeled placeholder frames
// because v0.0.1 does not ship a published Figma component library.
//
// v0.0.2 will replace this with a real DS runtime loader that resolves refs
// against the user's Figma library.

const __cdFonts = [
  { family: "Inter", style: "Regular" },
  { family: "Inter", style: "Semi Bold" },
  { family: "Inter", style: "Bold" },
];

const __cdColorMap = {
  "color.bg.base":          { r: 1, g: 1, b: 1 },
  "color.bg.elevated":      { r: 0.97, g: 0.97, b: 0.98 },
  "color.text.primary":     { r: 0.09, g: 0.10, b: 0.12 },
  "color.text.secondary":   { r: 0.42, g: 0.45, b: 0.50 },
  "color.accent.primary":   { r: 0.22, g: 0.47, b: 0.95 },
  "color.feedback.error":   { r: 0.90, g: 0.25, b: 0.25 },
  "color.feedback.success": { r: 0.18, g: 0.65, b: 0.36 },
  "color.border.subtle":    { r: 0.87, g: 0.88, b: 0.90 },
};

const __cdTypoMap = {
  "typo.title.lg":   { size: 28, weight: "Semi Bold", lineHeight: 36 },
  "typo.title.md":   { size: 20, weight: "Semi Bold", lineHeight: 28 },
  "typo.body.md":    { size: 16, weight: "Regular",   lineHeight: 24 },
  "typo.caption.sm": { size: 14, weight: "Regular",   lineHeight: 20 },
};

const __cdSpaceMap = {
  "space.xs":  4,
  "space.sm":  8,
  "space.md":  16,
  "space.lg":  24,
  "space.xl":  32,
};

async function __cdPreloadFonts(_dsRef) {
  for (const f of __cdFonts) {
    try { await figma.loadFontAsync(f); } catch (_) {}
  }
}

async function __cdBindFill(node, ref) {
  const rgb = __cdColorMap[ref];
  if (!rgb) return;
  if ("fills" in node) node.fills = [{ type: "SOLID", color: rgb }];
}

async function __cdBindTextStyle(textNode, ref) {
  const t = __cdTypoMap[ref] || __cdTypoMap["typo.body.md"];
  try {
    textNode.fontName = { family: "Inter", style: t.weight };
  } catch (_) {
    textNode.fontName = { family: "Inter", style: "Regular" };
  }
  textNode.fontSize = t.size;
  textNode.lineHeight = { value: t.lineHeight, unit: "PIXELS" };
}

async function __cdBindPadding(frame, ref) {
  const p = __cdSpaceMap[ref] ?? 16;
  frame.paddingTop = p; frame.paddingBottom = p;
  frame.paddingLeft = p; frame.paddingRight = p;
}

async function __cdBindGap(frame, ref) {
  frame.itemSpacing = __cdSpaceMap[ref] ?? 8;
}

async function __cdCreateInstance(componentRef) {
  // v0.0.1: no published component library available. Stand up a labeled
  // placeholder frame so the user still sees structure.
  const ph = figma.createFrame();
  ph.name = componentRef;
  ph.layoutMode = "HORIZONTAL";
  ph.counterAxisAlignItems = "CENTER";
  ph.primaryAxisAlignItems = "CENTER";
  ph.paddingTop = 12; ph.paddingBottom = 12;
  ph.paddingLeft = 16; ph.paddingRight = 16;
  ph.resize(240, 44);
  ph.fills = [{ type: "SOLID", color:
    componentRef.startsWith("button.primary")   ? __cdColorMap["color.accent.primary"] :
    componentRef.startsWith("button.secondary") ? __cdColorMap["color.bg.elevated"] :
    componentRef.startsWith("input.")           ? __cdColorMap["color.bg.elevated"] :
                                                   __cdColorMap["color.bg.elevated"]
  }];
  ph.cornerRadius = 8;
  ph.strokes = componentRef.startsWith("input.")
    ? [{ type: "SOLID", color: __cdColorMap["color.border.subtle"] }]
    : [];
  ph.strokeWeight = componentRef.startsWith("input.") ? 1 : 0;

  // stash componentRef on the node so __cdApplyProps can render a label
  ph.setPluginData("componentRef", componentRef);
  return ph;
}

async function __cdApplyProps(inst, props) {
  const label = props && (props.label || props.placeholder || props.text);
  if (!label) return;
  const ref = inst.getPluginData("componentRef") || "";
  const t = figma.createText();
  try { t.fontName = { family: "Inter", style: "Semi Bold" }; } catch (_) {}
  t.characters = String(label);
  t.fontSize = 14;
  t.fills = [{ type: "SOLID", color:
    ref.startsWith("button.primary") ? { r: 1, g: 1, b: 1 } :
                                        __cdColorMap["color.text.primary"]
  }];
  inst.appendChild(t);
}

async function __cdPlacePage(frame, _pageName) {
  figma.currentPage.appendChild(frame);
}

function __cdEmitResult(result) {
  console.log("codedesign result:", result);
  const n = (result.createdNodeIds || []).length;
  const errs = (result.errors || []).length;
  const msg = errs === 0
    ? `codedesign: created ${n} nodes`
    : `codedesign: created ${n} nodes, ${errs} error(s) — see console`;
  figma.notify(msg);
  figma.closePlugin();
}

// -----------------------------------------------------------------------------
// end runtime shim
// -----------------------------------------------------------------------------
