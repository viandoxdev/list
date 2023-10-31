// vi: shiftwidth=2 softtabstop=2

import { createSignal, Index, Match, Show, Switch } from 'solid-js'
import { FiMenu, FiTriangle, FiX } from 'solid-icons/fi'
import Editing from './Editing.tsx'
import './App.css'
import { createStore, produce } from 'solid-js/store';
import { animationFrame, time } from './utils.tsx'
import { Api, Item } from './api.ts'
import ItemComponent from './Item.tsx';
import { VsTriangleRight } from 'solid-icons/vs';

interface EditingInfo {
  listIndex: number,
  itemIndex: number,
  value: string,
  new: boolean,
}

interface OwningList {
  id: number,
  name: string,
  items: Item[]
}

function App() {
  const [lists, setLists] = createStore<OwningList[]>([]);
  const [sidebarOpen, setSidebarOpen] = createSignal(false);
  const [editing, setEditing] = createSignal<undefined | EditingInfo>(undefined);
  const [currentListIndex, setCurrentListIndex] = createSignal(0);
  const currentList = () => lists[currentListIndex()];

  const api = new Api("http://localhost:9000", "listclient", "7Q8cA87Wd5xHYPqxQe5AjeF9WSnX7hSzMjmZRA6T9r3BZAqSzcWXAVrpwg6qT247CHEBpUhemfKm36YF8AShV654Tg8HtK36TgcJX4bPUkaEZdfgn2wEXhQrHbYgmwKf3TLHAfJMRbsmCBjkUNxfTVHCuUqvbCApvPFdgDzfwYqXj4uszWrzzhGmqshZbNnxXGHDytgkp6gsetQMEn5EgY6J6WwADBHf6G6hCBUAYLAdyPpaBMbJFxBugEjVfcmD");

  (async () => {
    const lists: OwningList[] = (await api.get_lists()).map(l => ({ ...l, items: [] as Item[] }));
    for (const item of await api.get_all_items()) {
      const list = lists.find(l => l.id === item.list_id);
      if (list) {
        list.items.push(item);
      } else {
        console.warn("Dangling item")
      }
    }

    setLists(lists);
  })();

  return (
    <>
      <div id="topbar">
        <button id="menu" onClick={() => setSidebarOpen(!sidebarOpen())}>
          <Show when={sidebarOpen()} fallback={
            <FiMenu size={26} />
          }>
            <FiX size={26} />
          </Show>
        </button>
        Liste
        <Show when={lists.length > 0}> 
          <span id="list_name"> {currentList().name} </span>
        </Show>
      </div>
      <div id="content" style={{ transform: `translateX(${sidebarOpen() ? "0" : "-40vw"})` }}>
        <div id="sidebar">
          <Index each={lists}>
            {(list, index) =>
              <div class="list" onClick={() => setCurrentListIndex(index)}>{list().name}</div>
            }
          </Index>
        </div>
        <Show when={lists.length > 0}>
          <div id="items">
            <Index each={currentList().items}>
              {(item, index) =>
                <ItemComponent
                  item={item()}
                  onEdit={(content) => setEditing({ listIndex: currentListIndex(), itemIndex: index, value: content, new: false, })}
                  onRemove={async () => {
                    const id = item().id;
                    setLists(produce((lists) => lists[currentListIndex()].items.splice(index, 1)));
                    (async () => {
                      await api.delete_item(id);
                    })();
                  }} />
              }
            </Index>
            <Show when={currentList().items.length === 0}>
              <div id="placeholder">
                La liste est vide, cliquez sur "+" pour ajouter un élément
              </div>
            </Show>
          </div>
        </Show>
      </div>
      <button id="add" onClick={() => {
        let index = currentList().items.length;
        setEditing({
          listIndex: currentListIndex(),
          itemIndex: index,
          value: "",
          new: true,
        });
      }}>
        +
      </button>
      <Show when={editing() !== undefined}>
        <Editing onCancel={() => setEditing(undefined)} onConfirm={(value) => {
          const edit = editing() as EditingInfo;
          const list_id = lists[edit.listIndex].id;
          setLists(produce((lists) => {
            if(edit.new) {
              lists[edit.listIndex].items.push({
                content: value,
                list_id,
                id: 0,
              })
            } else {
              lists[edit.listIndex].items[edit.itemIndex].content = value;
            }
          }));
          (async () => {
            if(edit.new) {
              const id = (await api.create_item(list_id, value)).id;
              setLists(produce((lists) => {
                lists[edit.listIndex].items[edit.itemIndex].id = id;
              }));
            } else {
              await api.edit_item(lists[edit.listIndex].items[edit.itemIndex].id, value);
            }
          })();
          setEditing(undefined)
        }} value={editing()?.value || ""}></Editing>
      </Show>
    </>
  )
}

export default App
