"use client";

import React from "react";
import { useRouter } from "./hooks";

export type LinkProps = Omit<
  React.AnchorHTMLAttributes<HTMLAnchorElement>,
  "href"
> & {
  href: string;
};

export function Link({ href, onClick, children, ...rest }: LinkProps) {
  const router = useRouter();

  return (
    <a
      href={href}
      onClick={(event) => {
        onClick?.(event);
        if (event.defaultPrevented) return;
        event.preventDefault();
        router.push(href);
      }}
      {...rest}
    >
      {children}
    </a>
  );
}

export default Link;
