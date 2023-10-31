// vi: shiftwidth=2 softtabstop=2

import { onCleanup, onMount } from 'solid-js';
import './Item.css'
import { animationFrame, time } from './utils';
import { Item } from './api';

interface ItemProps {
  item: Item,
  onEdit: (content: string) => any,
  onRemove: () => any,
}

function getTouchByIdent(touches: TouchList, ident: number | undefined): Touch | null {
  if (ident === undefined) return null;
  for (let i = 0; i < touches.length; i++) {
    const touch = touches.item(i);
    if (touch?.identifier === ident) {
      return touch;
    }
  }
  return null;
}

enum Direction {
  Vertical,
  Horizontal,
}

function ItemComponent(props: ItemProps) {
  let div: HTMLDivElement;
  // Last position of the touch
  let lastPos = { x: 0, y: 0 };
  // Start position of the touch
  let startPos = { x: 0, y: 0 };
  // Identifier of the touch, to make sure we track the same troughout the different events
  let touchIdent: undefined | number = undefined;
  // Manhattan distance traveled since the start, used to know if we should trigger a click or a swipe
  // and when to check the overall direction of the swipe
  let traveled = 0;
  // Direction of the swipe if known
  let dir: Direction | undefined;
  // Offset in pixels of the element (for swipe)
  let offset = 0;
  const onTouchStart = (e: TouchEvent) => {
    if (touchIdent) return;
    e.preventDefault();
    if (e.targetTouches.length > 1) return;
    const touch = e.targetTouches.item(0);
    if (touch) {
      touchIdent = touch.identifier;
      lastPos.x = touch.clientX;
      lastPos.y = touch.clientY;
      startPos.x = touch.clientX;
      startPos.y = touch.clientY;
      traveled = 0;
      dir = undefined;
    }
  }
  const onTouchMove = (e: TouchEvent) => {
    const touch = getTouchByIdent(e.touches, touchIdent);
    if (touch === null) return;

    const dx = touch.clientX - lastPos.x;
    const dy = touch.clientY - lastPos.y;

    traveled += Math.abs(dx) + Math.abs(dy);

    if (traveled > 2 && dir === undefined) {
      dir = Math.abs(touch.clientX - startPos.x) > Math.abs(touch.clientY - startPos.y) ? Direction.Horizontal : Direction.Vertical;
      div.style.touchAction = dir === Direction.Horizontal ? "none" : "";
    }

    if (dir === Direction.Horizontal) {
      offset += dx;
      div.style.transform = `translateX(${offset}px)`;
      div.style.opacity = (1 - Math.abs(offset) / window.innerWidth).toString();
    }

    lastPos.x = touch.clientX;
    lastPos.y = touch.clientY;
  }
  const onTouchEnd = async (e: TouchEvent) => {
    const touch = getTouchByIdent(e.changedTouches, touchIdent);
    let removed = false;
    if (touch) {
      if (traveled < 10) {
        // We barely moved since the start of the touch, this is a click
        props.onEdit(props.item.content);
      }
      if (Math.abs(offset) >= window.innerWidth * 0.3) {
        removed = true;
      }
    }
    touchIdent = undefined;
    div.style.transition = "all 0.1s ease-out";
    div.style.touchAction = "";
    if (!removed) {
      div.style.opacity = "1";
      div.style.transform = 'translateX(0px)';
      await time(100);
      div.style.transition = "";
    } else {
      const size = div.getBoundingClientRect();
      await animationFrame();
      div.style.maxHeight = size.height + "px";
      await animationFrame();
      await animationFrame();
      div.style.maxHeight = "0px"
      div.style.padding = "0";
      div.style.opacity = "0";
      div.style.transform = `translateX(${offset < 0 ? "-" : ""}100%)`;
      await animationFrame();
      await time(100);
      div.style.transition = "";
      div.style.maxHeight = "";
      div.style.padding = "";
      div.style.opacity = "";
      div.style.transform = "";
      props.onRemove();
    }
    offset = 0;
  }
  const onTouchCancel = (_: TouchEvent) => {
    touchIdent = undefined;
    offset = 0;
    div.style.transform = 'translateX(10px)';
    div.style.touchAction = "";
    div.style.opacity = "1";
  }
  onMount(() => {
    document.addEventListener("touchmove", onTouchMove);
    document.addEventListener("touchend", onTouchEnd);
    document.addEventListener("touchcancel", onTouchCancel);
  })
  onCleanup(() => {
    document.removeEventListener("touchmove", onTouchMove);
    document.removeEventListener("touchend", onTouchEnd);
    document.removeEventListener("touchcancel", onTouchCancel);
  })
  return (
    <>
      <div ref={div} class="item" onTouchStart={onTouchStart}>
        {props.item.content}
      </div>
    </>
  )
}

export default ItemComponent
