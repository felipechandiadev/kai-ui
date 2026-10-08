"use client";

import { Link, usePathname } from "../../navigation";
import "./tabs.css";

export interface TabItem {
  url: string;
  label: string;
}

export interface TabsProps {
  items: TabItem[];
  activeTab?: string;
}

const Tabs: React.FC<TabsProps> = ({ items, activeTab }) => {
  const pathname = usePathname();

  const getIsActive = (tabUrl: string) => {
    if (activeTab) {
      return tabUrl === activeTab;
    }
    return pathname.includes(tabUrl.split("/").filter(Boolean).pop() || "");
  };

  return (
    <nav className="flex flex-wrap" data-test-id="tabs-root">
      {items.map((tab) => {
        const isActive = getIsActive(tab.url);
        return (
          <Link
            key={tab.url}
            href={tab.url}
            className={`fs-tabs__link whitespace-nowrap ${
              isActive ? "fs-tabs__link--active" : "fs-tabs__link--inactive"
            }`}
            aria-current={isActive ? "page" : undefined}
          >
            {tab.label}
          </Link>
        );
      })}
    </nav>
  );
};

export default Tabs;
