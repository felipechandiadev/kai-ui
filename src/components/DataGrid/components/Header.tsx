'use client'
import React, { useState, useCallback, useMemo } from 'react';
import { Plus, Search } from 'lucide-react';
import IconButton from '../../IconButton';
import { ButtonPill } from '../../Button';
import type { DataGridAddButtonVariant } from '../DataGrid';
import Toolbar from './Toolbar';
import TextField from '../../TextField';
import { useSearchParams, useRouter } from '../../../navigation';
import type { DataGridColumn } from '../DataGrid';
import Dialog from '../../Dialog';
import styles from './Header.module.css';
import { flattenHeaderActions } from '../utils/flattenHeaderActions';
import {
  getActionSlotPositions,
  HEADER_GRID_COLUMNS,
  HEADER_GRID_TEMPLATE_COLUMNS,
} from '../utils/headerGridPlacement';

interface HeaderProps {
  title?: string;
  filterMode?: boolean;
  onToggleFilterMode?: () => void;
  columns?: DataGridColumn[];
  createForm?: React.ReactNode;
  createFormTitle?: string;
  onAddClick?: () => void;
  addDisabled?: boolean;
  addButtonVariant?: DataGridAddButtonVariant;
  addButtonLabel?: string;
  screenWidth?: number;
  onExportExcel?: () => Promise<void>;
  headerActions?: React.ReactNode;
  showSortButton?: boolean;
  showFilterButton?: boolean;
  showExportButton?: boolean;
  showSearch?: boolean;
  onSearchChange?: (value: string) => void;
}

function HeaderSearchField({
  name,
  value,
  onChange,
  className = '',
  testId,
}: {
  name: string;
  value: string;
  onChange: (e: React.ChangeEvent<HTMLInputElement | HTMLTextAreaElement>) => void;
  className?: string;
  testId: string;
}) {
  return (
    <TextField
      label=""
      name={name}
      value={value}
      onChange={onChange}
      density="compact"
      startAdornment={<Search className="h-4 w-4 shrink-0 text-secondary" aria-hidden />}
      className={className}
      aria-label="Buscar"
      data-test-id={testId}
    />
  );
}

const Header: React.FC<HeaderProps> = ({
  title,
  filterMode = false,
  onToggleFilterMode,
  columns = [],
  createForm,
  createFormTitle,
  onAddClick,
  addDisabled = false,
  addButtonVariant = 'icon',
  addButtonLabel,
  onExportExcel,
  headerActions,
  showSortButton = true,
  showFilterButton = true,
  showExportButton = true,
  showSearch = true,
  onSearchChange,
}) => {
  const router = useRouter();
  const searchParams = useSearchParams();
  const [isCreateModalOpen, setIsCreateModalOpen] = useState(false);
  const [searchInput, setSearchInput] = useState(searchParams.get('search') || '');

  React.useEffect(() => {
    setSearchInput(searchParams.get('search') || '');
  }, [searchParams]);

  const debounceTimer = React.useRef<NodeJS.Timeout | null>(null);

  const handleChange = useCallback((e: React.ChangeEvent<HTMLInputElement | HTMLTextAreaElement>) => {
    const value = e.target.value;
    setSearchInput(value);

    if (debounceTimer.current) {
      clearTimeout(debounceTimer.current);
    }

    debounceTimer.current = setTimeout(() => {
      if (onSearchChange) {
        onSearchChange(value);
      } else {
        const params = new URLSearchParams(searchParams.toString());
        if (value) {
          params.set('search', value);
        } else {
          params.delete('search');
        }
        params.set('page', '1');
        router.replace(`?${params.toString()}`);
      }
    }, 300);
  }, [searchParams, router, onSearchChange]);

  const showAdd = Boolean(createForm || onAddClick);
  const actionItems = useMemo(
    () => flattenHeaderActions(headerActions),
    [headerActions],
  );

  const fullWidthFlags = useMemo(
    () =>
      actionItems.map((item) => {
        if (!React.isValidElement(item)) return false;
        const props = item.props as Record<string, unknown>;
        return props["data-header-full-width"] === true;
      }),
    [actionItems],
  );

  const actionSlots = useMemo(
    () => getActionSlotPositions(actionItems.length, fullWidthFlags),
    [actionItems.length, fullWidthFlags],
  );

  const gridRowCount = useMemo(() => {
    if (actionSlots.length === 0) {
      return 1;
    }
    return Math.max(...actionSlots.map((slot) => slot.row));
  }, [actionSlots]);

  const openCreate = onAddClick || (() => setIsCreateModalOpen(true));

  const addTitleCell = (
    <div
      className="flex min-w-0 items-center gap-2 self-center"
      data-test-id="data-grid-header-add-title"
    >
      {showAdd ? (
        <div
          className={
            addButtonVariant === 'pillOutlined'
              ? 'flex min-w-0 shrink-0 items-center overflow-visible'
              : 'flex shrink-0 items-center'
          }
        >
          {addButtonVariant === 'pillOutlined' ? (
            <ButtonPill
              type="button"
              variant="outlined"
              disabled={addDisabled}
              onClick={openCreate}
              className="!inline-flex !w-auto !max-w-none items-center gap-1.5 whitespace-nowrap"
              data-test-id="add-button-pill"
              aria-label={
                addButtonLabel?.trim()
                  ? `Agregar ${addButtonLabel.trim()}`
                  : 'Agregar'
              }
            >
              <Plus className="h-4 w-4 shrink-0 text-secondary" aria-hidden />
              <span>{addButtonLabel?.trim() ? addButtonLabel.trim() : 'Agregar'}</span>
            </ButtonPill>
          ) : (
            <IconButton
              icon="Plus"
              variant="action"
              size="md"
              onClick={openCreate}
              disabled={addDisabled}
              data-test-id="add-button"
              aria-label="Agregar"
            />
          )}
        </div>
      ) : null}
      {title?.trim() ? (
        <div className="min-w-0 truncate text-lg font-semibold text-foreground">
          {title.trim()}
        </div>
      ) : null}
    </div>
  );

  const renderToolbarZone = (className: string) => (
    <div
      className={className}
      data-test-id="data-grid-header-toolbar-zone"
    >
      <Toolbar
        filterMode={filterMode}
        onToggleFilterMode={onToggleFilterMode}
        columns={columns}
        title={title}
        onExportExcel={onExportExcel}
        showSortButton={showSortButton}
        showFilterButton={showFilterButton}
        showExportButton={showExportButton}
      />
      {showSearch ? (
        <HeaderSearchField
          name="datagrid-search"
          value={searchInput}
          onChange={handleChange}
          className="w-full min-w-[10rem] max-w-[16rem]"
          testId="data-grid-search-input"
        />
      ) : null}
    </div>
  );

  return (
    <div className="w-full pb-4" data-test-id="data-grid-header">
      {/* Desktop: CSS grid (md+) */}
      <div
        className={styles.desktopGrid}
        style={{
          gridTemplateColumns: HEADER_GRID_TEMPLATE_COLUMNS,
          gridTemplateRows: `repeat(${gridRowCount}, auto)`,
        }}
        data-test-id="data-grid-header-grid"
      >
        <div
          className="min-w-0"
          style={{ gridRow: 1, gridColumn: 1 }}
        >
          {addTitleCell}
        </div>

        <div
          className="min-w-0"
          style={{ gridRow: 1, gridColumn: HEADER_GRID_COLUMNS }}
        >
          {renderToolbarZone(styles.toolbarZoneDesktop)}
        </div>

        {actionItems.map((item, index) => {
          const slot = actionSlots[index];
          if (!slot) {
            return null;
          }
          return (
            <div
              key={index}
              className="flex min-w-0 items-center self-center"
              style={{
                gridRow: slot.row,
                gridColumn: slot.colSpan
                  ? `${slot.col} / span ${slot.colSpan}`
                  : slot.col,
              }}
              data-test-id={`header-action-slot-${index}`}
            >
              {item}
            </div>
          );
        })}
      </div>

      {/* Mobile / sm: stack alineado a la izquierda (< md) */}
      <div className={styles.mobileStack} data-test-id="data-grid-header-mobile">
        {addTitleCell}

        {renderToolbarZone(styles.toolbarZoneMobile)}

        {actionItems.length > 0 ? (
          <div
            className={styles.mobileActions}
            data-test-id="header-actions-slot-mobile"
          >
            {actionItems.map((item, index) => (
              <div key={index} className="min-w-0 max-w-full">
                {item}
              </div>
            ))}
          </div>
        ) : null}
      </div>

      {createForm ? (
        <Dialog
          open={isCreateModalOpen}
          onClose={() => setIsCreateModalOpen(false)}
          size="lg"
          scroll="body"
          hideActions={true}
          title={createFormTitle}
        >
          {React.isValidElement(createForm)
            ? React.cloneElement(createForm, {
                onClose: () => setIsCreateModalOpen(false),
              } as React.Attributes & { onClose?: () => void })
            : createForm}
        </Dialog>
      ) : null}
    </div>
  );
};

export default Header;
