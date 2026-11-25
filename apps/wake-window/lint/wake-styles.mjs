const COLOUR =
  /#[0-9a-f]{3,8}\b|\b(?:rgb|rgba|hsl|hsla|oklch|oklab|lab|lch|color)\(|\b(?:black|white|red|green|blue|gray|grey|yellow|orange|purple|pink|brown|cyan|magenta|silver|navy|teal|maroon|olive|lime|aqua|fuchsia)\b/iu;
const TOKEN_ONLY = new Set([
  "borderBottomLeftRadius",
  "borderBottomRightRadius",
  "borderRadius",
  "borderTopLeftRadius",
  "borderTopRightRadius",
  "boxShadow",
  "fontFamily",
]);
const STYLE_CALLS = new Set(["create", "keyframes"]);

const textOf = (node) => {
  if (node.type === "Literal" && typeof node.value === "string") {
    return node.value;
  }
  if (node.type === "TemplateLiteral") {
    return node.quasis.map((quasi) => quasi.value.raw).join(" ");
  }
  return null;
};

const isStyleCall = (node) =>
  node.type === "CallExpression" &&
  node.callee.type === "MemberExpression" &&
  node.callee.object.type === "Identifier" &&
  node.callee.object.name === "stylex" &&
  node.callee.property.type === "Identifier" &&
  STYLE_CALLS.has(node.callee.property.name);

const insideStyleCall = (node) => {
  let current = node.parent;
  while (current) {
    if (isStyleCall(current)) {
      return true;
    }
    current = current.parent;
  }
  return false;
};

const keyOf = (property) =>
  property.key.type === "Identifier" ? property.key.name : "";

const noRawStyleValues = {
  create: (context) => ({
    Property: (node) => {
      if (!insideStyleCall(node)) {
        return;
      }
      const text = textOf(node.value);
      if (text === null) {
        return;
      }
      if (TOKEN_ONLY.has(keyOf(node)) && node.value.type === "Literal") {
        context.report({
          message: `${keyOf(node)} comes from the Keel token module, not a literal.`,
          node: node.value,
        });
      }
      if (COLOUR.test(text)) {
        context.report({
          message: "Colours come from the Keel token module.",
          node: node.value,
        });
      }
      if (text.includes("!important")) {
        context.report({ message: "No !important.", node: node.value });
      }
    },
  }),
  meta: { schema: [], type: "problem" },
};

const keelClassNames = {
  create: (context) => ({
    JSXAttribute: (node) => {
      if (node.name.name === "style") {
        context.report({
          message: "No style prop: use stylex.props.",
          node,
        });
      }
      if (node.name.name !== "className") {
        return;
      }
      const { value } = node;
      const written =
        value && value.type === "Literal" && typeof value.value === "string"
          ? value.value.split(/\s+/u)
          : [""];
      if (!written.every((name) => name.startsWith("k-"))) {
        context.report({
          message:
            "className is written by hand only for Keel's k- classes; use stylex.props for the rest.",
          node,
        });
      }
    },
  }),
  meta: { schema: [], type: "problem" },
};

export default {
  meta: { name: "wake-styles" },
  rules: {
    "keel-class-names": keelClassNames,
    "no-raw-style-values": noRawStyleValues,
  },
};
