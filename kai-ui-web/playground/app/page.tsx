import { DataGrid, DataGridTable, NextNavProvider, type DataGridColumn } from "@felipechandiadev/ui";

export const dynamic = "force-dynamic";

const columns: DataGridColumn[] = [
  { field: "name", headerName: "Nombre", flex: 1 },
  { field: "city", headerName: "Ciudad", width: 180 },
];

const serverRows = [
  { id: "s1", name: "Fila servidor Alpha", city: "Santiago" },
  { id: "s2", name: "Fila servidor Beta", city: "Valparaíso" },
];

const clientRows = [
  { id: "c1", name: "Fila cliente Gamma", city: "Concepción" },
  { id: "c2", name: "Fila cliente Delta", city: "La Serena" },
];

type SearchParams = Record<string, string | string[] | undefined>;

function toQuery(params: SearchParams): string {
  const search = new URLSearchParams();
  for (const [key, value] of Object.entries(params)) {
    if (typeof value === "string") search.set(key, value);
    else if (Array.isArray(value)) value.forEach((item) => search.append(key, item));
  }
  return search.toString();
}

export default async function Page({
  searchParams,
}: {
  searchParams: Promise<SearchParams>;
}) {
  const search = toQuery(await searchParams);

  return (
    <NextNavProvider search={search}>
      <main className="flex flex-col gap-10 p-6">
        <section>
          <h1 className="mb-3 text-lg font-semibold">DataGrid directo</h1>
          <DataGridTable
            columns={columns}
            rows={serverRows}
            totalRows={50}
            limit={25}
            title="Render de servidor"
            data-test-id="datagrid-ssr"
            showExportButton={false}
            showSortButton={false}
          />
        </section>
        <section>
          <h1 className="mb-3 text-lg font-semibold">DataGridWrapper</h1>
          <DataGrid
            columns={columns}
            rows={clientRows}
            totalRows={50}
            limit={25}
            title="Sin SSR"
            data-test-id="datagrid-wrapper"
            showExportButton={false}
            showSortButton={false}
          />
        </section>
      </main>
    </NextNavProvider>
  );
}
