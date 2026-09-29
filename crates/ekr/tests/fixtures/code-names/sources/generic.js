export function names(document) {
  return document["nodes"].map((node) => node["name"]);
}
