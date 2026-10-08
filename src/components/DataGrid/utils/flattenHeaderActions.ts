import React from "react";

type ElementWithChildren = React.ReactElement<{ children?: React.ReactNode }>;

function isFragment(node: React.ReactNode): node is React.ReactElement {
  return React.isValidElement(node) && node.type === React.Fragment;
}

function getElementChildren(node: React.ReactElement): React.ReactNode[] {
  const props = node.props as { children?: React.ReactNode };
  return React.Children.toArray(props.children);
}

function unwrapSingleWrapper(
  nodes: React.ReactNode[],
): React.ReactNode[] {
  if (nodes.length !== 1) {
    return nodes;
  }
  const only = nodes[0];
  if (!React.isValidElement(only)) {
    return nodes;
  }
  if (only.type === React.Fragment) {
    return getElementChildren(only);
  }
  if (typeof only.type === "string") {
    const childArray = getElementChildren(only as ElementWithChildren);
    if (childArray.length > 1) {
      return childArray;
    }
  }
  return nodes;
}

/**
 * Normaliza `headerActions` a una lista de ítems (uno por celda del grid).
 * Soporta array, Fragment, o un único wrapper DOM con varios hijos.
 */
export function flattenHeaderActions(node: React.ReactNode): React.ReactNode[] {
  if (node == null || node === false) {
    return [];
  }
  const flat = React.Children.toArray(node).flatMap((child) => {
    if (isFragment(child)) {
      return getElementChildren(child);
    }
    return [child];
  });
  return unwrapSingleWrapper(flat).filter(
    (item) => item != null && item !== false,
  );
}
