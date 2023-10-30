// vi: shiftwidth=2 softtabstop=2
import { onMount } from 'solid-js';
import './Editing.css'

interface EditingProps {
  onConfirm: (value: string) => any,
  onCancel: () => any,
  value: string,
}

function Editing(props: EditingProps) {
  let input: HTMLInputElement;
  let overlay: HTMLDivElement;
  onMount(() => requestAnimationFrame(() => requestAnimationFrame(() => {
    overlay.style.opacity = "1";
  })))
  return (
    <>
      <div id="overlay" ref={overlay}>
        <form id="popup" onSubmit={e => e.preventDefault()}>
          <input ref={input} type="text" value={props.value} />
          <div id="controls">
            <input type="button" onClick={props.onCancel} value="annuler" />
            <input type="submit" onClick={() => input.value !== "" && props.onConfirm(input.value)} value="confirmer" />
          </div>
        </form>
      </div>
    </>
  )
}

export default Editing
