// vi: shiftwidth=2 softtabstop=2

import { createResource, createSignal, Index, Show, Suspense } from 'solid-js'
import { FiMenu, FiPlus, FiX } from 'solid-icons/fi'
import Editing from './Editing.tsx'
import './App.css'
import { createStore, produce, reconcile } from 'solid-js/store';
import { Api, Item, List } from './api.ts'
import ItemComponent from './Item.tsx';
import { VsLoading } from 'solid-icons/vs';

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

interface WsMessageList {
  tag: "ListCreated" | "ListRenamed" | "ListRemoved",
  value: List
}
interface WsMessageItem {
  tag: "ItemCreated" | "ItemEdited" | "ItemRemoved",
  value: Item
}
type WsMessage = WsMessageList | WsMessageItem;

function App() {
  const host = "localhost:9000"
  const api = new Api(`http://${host}`, "listclient", "7Q8cA87Wd5xHYPqxQe5AjeF9WSnX7hSzMjmZRA6T9r3BZAqSzcWXAVrpwg6qT247CHEBpUhemfKm36YF8AShV654Tg8HtK36TgcJX4bPUkaEZdfgn2wEXhQrHbYgmwKf3TLHAfJMRbsmCBjkUNxfTVHCuUqvbCApvPFdgDzfwYqXj4uszWrzzhGmqshZbNnxXGHDytgkp6gsetQMEn5EgY6J6WwADBHf6G6hCBUAYLAdyPpaBMbJFxBugEjVfcmD");

  const [lists, setLists] = createStore<OwningList[]>([]);
  const [listsResource] = createResource(async () => {
    const lists: OwningList[] = (await api.get_lists()).map(l => ({ ...l, items: [] as Item[] }));
    for (const item of await api.get_all_items()) {
      const list = lists.find(l => l.id === item.list_id);
      if (list) {
        list.items.push(item);
      } else {
        console.warn("Dangling item")
      }
    }
    return lists
  }, {
    storage: () => [
      () => lists,
      ((value: any) => setLists(reconcile(value()))) as any
    ],
    initialValue: lists,
  });

  const [sidebarOpen, setSidebarOpen] = createSignal(false);
  const [editing, setEditing] = createSignal<undefined | EditingInfo>(undefined);
  const [currentListIndex, setCurrentListIndex] = createSignal(0);

  // Websocket
  const ws = new WebSocket(`ws://${host}/ws`);
  ws.onmessage = async e => {
    const data: WsMessage = JSON.parse(await e.data.text());
    switch (data.tag) {
      case "ListCreated": {
        if (lists.find(l => l.id === data.value.id)) break;
        const index = lists.length;
        setLists(index, { ...data.value, items: [] })
        break;
      }
      case "ItemCreated": {
        const list_index = lists.findIndex(l => l.id === data.value.list_id);
        // Shouldn't happen, but there just in case
        if (list_index === -1) {
          console.warn("Created item that doesn't belong to any list");
          break;
        }
        const list = lists[list_index];
        // If we are the one who created the item, there is an item with id 0 we need to change
        let index = list.items.findIndex(i => i.id === 0);
        // Otherwise, we have to create a new item
        index < 0 && (index = list.items.length);
        // Shouldn't happen, but just in case
        if (list && list.items.find(i => i.id === data.value.id)) break;
        setLists(list_index, "items", index, data.value);
        break;
      }
      case "ListRenamed": {
        const index = lists.findIndex(l => l.id === data.value.id);
        if(index === -1) {
          console.warn("Renamed list that doesn't exist");
          break;
        }
        setLists(index, "name", data.value.name);
        break;
      }
      case "ItemEdited": {
        const list_index = lists.findIndex(l => l.id === data.value.list_id);
        // Shouldn't happen, but there just in case
        if (list_index === -1) {
          console.warn("Edited item that doesn't belong to any list");
          break;
        }
        const index = lists[list_index].items.findIndex(i => i.id === data.value.id);
        if (index === -1) {
          console.warn("Edited item that doesn't exist");
          break;
        }
        setLists(list_index, "items", index, "content", data.value.content);
        break;
      }
      case "ListRemoved": {
        setLists(lists => lists.filter(l => l.id !== data.value.id));
        break;
      }
      case "ItemRemoved": {
        const list_index = lists.findIndex(l => l.id === data.value.list_id);
        if(list_index === -1) {
          console.warn("Deleted an item that doesn't belong to any list");
          break;
        }
        const index = lists[list_index].items.findIndex(i => i.id === data.value.id);
        // We might have already deleted that item
        if(index === -1) break;
        setLists(produce(lists => {
          lists[list_index].items.splice(index, 1);
        }));
        break;
      }
    }
  }

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
          <span id="list_name"> {lists[currentListIndex()].name} </span>
        </Show>
      </div>
      <Suspense fallback={
        <div id="loading">
          <VsLoading size="60" id="spinner"/>
          <span>Chargement...</span>
        </div>
      }>
        <div id="content" style={{ transform: `translateX(${sidebarOpen() ? "0" : "-40vw"})` }}>
          <div id="sidebar">
            <Index each={listsResource()}>
              {(list, index) =>
                <div class="list" onClick={() => setCurrentListIndex(index)}>{list().name}</div>
              }
            </Index>
          </div>
          <Show when={lists.length > 0}>
            <div id="items">
              <Index each={lists[currentListIndex()].items}>
                {(item, index) =>
                  <ItemComponent
                    item={item()}
                    onEdit={(content) => setEditing({ listIndex: currentListIndex(), itemIndex: index, value: content, new: false, })}
                    onRemove={async () => {
                      const id = item().id;
                      setLists(produce(list => list[currentListIndex()].items.splice(index, 1)))
                      api.delete_item(id);
                    }} />
                }
              </Index>
              <Show when={lists[currentListIndex()].items.length === 0}>
                <div id="placeholder">
                  La liste est vide, cliquez sur "+" pour ajouter un élément
                </div>
              </Show>
            </div>
          </Show>
        </div>
      </Suspense>
      <button id="add" onClick={() => {
        let index = lists[currentListIndex()].items.length;
        setEditing({
          listIndex: currentListIndex(),
          itemIndex: index,
          value: "",
          new: true,
        });
      }}>
        <FiPlus size="30"/>
      </button>
      <Show when={editing() !== undefined}>
        <Editing onCancel={() => setEditing(undefined)} onConfirm={(value) => {
          const edit = editing() as EditingInfo;
          const list_id = lists[edit.listIndex].id;
          const item = edit.new ? {
            content: value,
            id: 0,
            list_id,
          } : lists[edit.listIndex].items[edit.itemIndex];
          setLists(edit.listIndex, "items", edit.itemIndex, item);
          if(edit.new) {
            api.create_item(list_id, value)
          } else {
            api.edit_item(item.id, value);
          }
          setEditing(undefined)
        }} value={editing()?.value || ""}></Editing>
      </Show>
    </>
  )
}

export default App
