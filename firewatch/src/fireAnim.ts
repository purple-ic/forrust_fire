import { elById, intervalWhenShown, nn } from "./util";

let progress = 0;
export const title =
    elById(HTMLHeadingElement, "title");
const str = nn(title.textContent);
title.textContent = "";

[...str].forEach(char => {
    const element = document.createElement("span");
    element.textContent = char;
    element.classList.add("fire-char");
    title.appendChild(element);
});
const els = title.childElementCount;

function setState(idx: number, orange: boolean, red: boolean) {
    const RNG = "fire-char-1";
    const RED = "fire-char-2";
    const cl = (title.childNodes[idx % els] as HTMLElement).classList;

    if (orange) {
        cl.add(RNG);
    } else {
        cl.remove(RNG);
    }

    if (red) {
        cl.add(RED);
    } else {
        cl.remove(RED);
    }
}

function update() {
    if (!title.checkVisibility()) {
        return;
    }

    setState(progress, false, false);

    progress += 1;
    progress %= els;

    setState(progress, true, false);
    setState(progress + 1, false, true);
    setState(progress + 2, true, false);
}

intervalWhenShown(update, 100);