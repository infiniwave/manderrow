import { faArrowDownShortWide, faArrowUpWideShort, faRefresh } from "@fortawesome/free-solid-svg-icons";
import { Fa } from "solid-fa";
import { createResource, createSignal, ResourceFetcherInfo, Show, useContext } from "solid-js";

import { countModIndex, fetchModIndex, ModSortColumn, queryModIndex, SortOption } from "../../api";
import { createProgressProxyStore } from "../../api/tasks";
import { numberFormatter } from "../../utils";

import { ErrorContext } from "../global/ErrorBoundary";
import { SimpleProgressIndicator } from "../global/Progress";
import SelectDropdown from "../global/SelectDropdown";
import { SortableList } from "../global/SortableList";
import TogglableDropdown from "../global/TogglableDropdown";
import ModList from "./ModList";

import styles from "./ModSearch.module.css";

export interface InitialProgress {
  completed_steps: null;
  total_steps: null;
  progress: null;
}

const MODS_PER_PAGE = 50;

export default function ModSearch(props: { game: string }) {
  const [query, setQuery] = createSignal("");

  const [sort, setSort] = createSignal<SortOption<ModSortColumn>[]>([
    { column: ModSortColumn.Relevance, descending: true },
    { column: ModSortColumn.Downloads, descending: true },
    { column: ModSortColumn.Name, descending: false },
    { column: ModSortColumn.Owner, descending: false },
  ]);

  const [progress, setProgress] = createProgressProxyStore();

  const reportErr = useContext(ErrorContext)!;

  const [loadStatus, { refetch: refetchModIndex }] = createResource(
    () => props.game,
    async (game, info: ResourceFetcherInfo<boolean, never>) => {
      try {
        await fetchModIndex(game, { refresh: info.refetching }, (event) => {
          if (event.event === "created") {
            setProgress(event.progress);
          }
        });
      } catch (e) {
        reportErr(e);
        throw e;
      }
      return true;
    },
  );

  const [queriedMods] = createResource(
    () => [props.game, query(), sort(), loadStatus.loading] as [string, string, SortOption[], true | undefined],
    async ([game, query, sort]) => {
      const count = await countModIndex(game, query);
      return {
        count,
        mods: async (page: number) =>
          (
            await queryModIndex(game, query, sort, {
              skip: page * MODS_PER_PAGE,
              limit: MODS_PER_PAGE,
            })
          ).mods,
      };
    },
    { initialValue: { mods: async (_: number) => [], count: 0 } },
  );

  const [profileSortOrder, setProfileSortOrder] = createSignal(false);

  return (
    <div class={styles.modSearch}>
      <form on:submit={(e) => e.preventDefault()} class={styles.modSearch__form}>
        <div class={styles.modSearch__searchBar}>
          <input
            type="mod-search"
            placeholder="Search for mods"
            value={query()}
            on:input={(e) => setQuery(e.target.value)}
          />
          <label for="mod-search" class="phantom">
            Mod search
          </label>

          <SelectDropdown
            label={{ labelText: "preset", preset: "Sort By" }}
            options={{
              [ModSortColumn.Relevance]: {
                value: "relevance",
                selected: true,
              },
              [ModSortColumn.Downloads]: {
                value: "downloads",
              },
              [ModSortColumn.Name]: {
                value: "name",
              },
              [ModSortColumn.Owner]: {
                value: "owner",
              },
            }}
            onChanged={() => {}}
          />

          <button
            type="button"
            // class={sidebarStyles.sidebar__profilesSearchSortByBtn}
            on:click={() => setProfileSortOrder((order) => !order)}
          >
            {profileSortOrder() ? <Fa icon={faArrowUpWideShort} /> : <Fa icon={faArrowDownShortWide} />}
          </button>

          <TogglableDropdown label="Advanced" labelClass={styles.modSearch__dropdownBtn}>
            <div class={styles.searchOptions}>
              <div class={styles.sortOptions}>
                <div class={styles.inner}>
                  <SortableList items={[sort, setSort]} id={(option) => option.column}>
                    {(option, i) => {
                      const id = `sort-descending-${option.column}`;
                      return (
                        <div class={styles.sortOption}>
                          {option.column}
                          <div class={styles.descendingToggle}>
                            <input
                              type="checkbox"
                              id={id}
                              checked={option.descending}
                              on:change={(e) =>
                                setSort([
                                  ...sort().slice(0, i),
                                  { column: option.column, descending: e.target.checked },
                                  ...sort().slice(i + 1),
                                ])
                              }
                            />
                            <label for={id}>{option.descending ? "Descending" : "Ascending"}</label>
                          </div>
                        </div>
                      );
                    }}
                  </SortableList>
                </div>
              </div>
            </div>
          </TogglableDropdown>
        </div>
      </form>

      <Show when={loadStatus.loading}>
        <div class={styles.progressLine}>
          <p>Fetching mods</p>
          <SimpleProgressIndicator progress={progress} />
        </div>
      </Show>

      <Show when={queriedMods.latest} fallback={<p>Querying mods...</p>}>
        <div class={styles.discoveredLine}>
          Discovered {numberFormatter.format(queriedMods()!.count)} mods
          <button class={styles.refreshButton} on:click={() => refetchModIndex()}>
            <Fa icon={faRefresh} />
          </button>
        </div>
        <ModList mods={queriedMods()!.mods} />
      </Show>
    </div>
  );
}
