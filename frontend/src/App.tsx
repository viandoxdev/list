// vi: shiftwidth=2 softtabstop=2

import { createSignal, Index, Match, Show, Switch } from 'solid-js'
import { FiMenu, FiX } from 'solid-icons/fi'
import Editing from './Editing.tsx'
import Item from './Item.tsx'
import './App.css'
import { createStore, produce } from 'solid-js/store';
import { animationFrame, time } from './utils.tsx'

interface EditingInfo {
  listIndex: number,
  itemIndex: number,
  value: string,
}

function App() {
  const [lists, setLists] = createStore([
    {
      name: "maison",
      items: [
        "chocolot",
        "RTX4090TI",
        "your mom",
        "world domination",
        "item",
        "Lorem Ipsum is simply dummy text of the printing and typesetting industry. Lorem Ipsum has been the industry's standard dummy text ever since the 1500s, when an unknown printer took a galley of type and scrambled it to make a type specimen book. It has survived not only five centuries, but also the leap into electronic typesetting, remaining essentially unchanged. It was popularised in the 1960s with the release of Letraset sheets containing Lorem Ipsum passages, and more recently with desktop publishing software like Aldus PageMaker including versions of Lorem Ipsum.",
        "a",
        "b",
        "c",
        "d",
        "e",
        "f",
        "g"
      ]
    },
    {
      name: "boutique",
      items: [
        "gruyere",
        "jason derulo",
        "evangelion",
        "italie",
        "lorem",
        "fortnite"
      ]
    },
    {
      name: "This is my reckoning, sdjffjkgnsddgfnjkdngdsfjgonsdjfnggkfsgn",
      items: []
    }
  ]);
  const [sidebarOpen, setSidebarOpen] = createSignal(false);
  const [editing, setEditing] = createSignal<undefined | EditingInfo>(undefined);
  const [currentListIndex, setCurrentListIndex] = createSignal(0);
  const currentList = () => lists[currentListIndex()].items;

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
      </div>
      <div id="content" style={{ transform: `translateX(${sidebarOpen() ? "0" : "-40vw"})` }}>
        <div id="sidebar">
          <Index each={lists}>
            {(list, index) =>
              <div class="list" onClick={() => setCurrentListIndex(index)}>{list().name}</div>
            }
          </Index>
        </div>
        <div id="items">
          <Index each={currentList()}>
            {(item, index) =>
              <Item 
                content={item()}
                onEdit={(content) => setEditing({ listIndex: currentListIndex(), itemIndex: index, value: content })}
                onRemove={async () => { 
                  setLists(produce((lists) => lists[currentListIndex()].items.splice(index, 1))) 
                }} />
            }
          </Index>
          <Show when={currentList().length === 0}>
            <div id="placeholder">
              La liste est vide, cliquez sur "+" pour ajouter un élément
            </div>
          </Show>
        </div>
      </div>
      <button id="add" onClick={() => {
        let index = currentList().length;
        setEditing({
          listIndex: currentListIndex(),
          itemIndex: index,
          value: "",
        });
      }}>
        +
      </button>
      <Show when={editing() !== undefined}>
        <Editing onCancel={() => setEditing(undefined)} onConfirm={(value) => {
          const edit = editing() as EditingInfo;
          setLists(produce((lists) => {
            lists[edit.listIndex].items[edit.itemIndex] = value;
          }));
          setEditing(undefined)
        }} value={editing()?.value || ""}></Editing>
      </Show>
    </>
  )
}

export default App
