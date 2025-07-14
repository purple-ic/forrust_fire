import { title } from "./fireAnim";
import { buildChild, elById, nn, randomElement, removeFromParent, TODO } from "./util";
import demo from "./demo.json";
import msgs from "./msgs.json";

import.meta.hot.accept;

interface Payload {
    level?: unknown;
    name?: unknown;
    ctx?: unknown;
    file?: unknown;
    is_span?: unknown;
    line?: unknown;
}

interface Tree {
    v?: Payload,
    [child: number]: Tree;
}

// for whatever reason, bun turns the below code:
/*
    if (payload.level != undefined) {
        const level = String(payload.level);
        const levelLo = level.toLowerCase();
*/
// into this code:
/*
    if (payload.level != null) {
      const level = undefined;
      const levelLo = level.toLowerCase();
*/
// which is very obviously bogus...
// for whatever reason (again), adding a function which simply returns `undefined` fixes this issue
function workAroundGetUndefined(): undefined {
    return undefined;
}

function buildHeader(parent: HTMLElement, payload: Payload) {
    if (payload.level != workAroundGetUndefined()) {
        const level = String(payload.level);
        const levelLo = level.toLowerCase();
        buildChild(parent, "span", el => {
            el.textContent = level + " ";
            el.classList.add("payload-level");
            el.classList.add("payload-level-" + levelLo);
        });

        // info.classList.add("payload-info-" + levelLo);
        // parent.classList.add("tree-node-" + level.toLowerCase());
    }
    if (payload.name != undefined) {
        buildChild(parent, "span", el => {
            el.textContent = String(payload.name);
            el.classList.add("payload-name");
        });
    }
}

function buildPayload(parent: HTMLElement, payload: Payload) {
    const div = document.createElement("div");
    div.classList.add("payload");

    function addKvPair(special: boolean, key: string, value: string) {
        const pairEl = document.createElement("p");
        pairEl.classList.add("payload-pair");
        buildChild(pairEl, "span", keyEl => {
            keyEl.textContent = key;
            keyEl.classList.add("payload-key");
            if (special)
                keyEl.classList.add("payload-key-special");
        }).insertAdjacentText("afterend", ": ");
        buildChild(pairEl, "span", valEl => {
            valEl.textContent = value;
            valEl.classList.add("payload-val");
        });

        div.appendChild(pairEl);
    }

    if (payload.is_span) {
        if (payload.file != undefined && payload.line != undefined)
            addKvPair(true, "at", String(payload.file) + ":" + String(payload.line));
    }

    const ctx = payload.ctx;
    if (ctx != undefined)
        for (const [key, value] of Object.entries(ctx)) {
            // const value = String(v);

            addKvPair(false, key, JSON.stringify(value));
        }

    if (parent.children[0] != undefined) {
        parent.insertBefore(div, parent.children[0]);
    } else {
        parent.appendChild(div);
    }
}

function buildTree(parent: HTMLElement, tree: Tree, level: number) {
    parent.classList.add("tree-node-" + level % 2);
    let children: (HTMLElement | null)[] = [];

    if (tree.v != undefined) {
        const payload = tree.v;
        buildChild(parent, "p", p => {
            p.classList.add("payload-info");
            buildHeader(p, payload);
        });
    }

    const inner = document.createElement("div");
    inner.classList.add("tree-node-inner");
    parent.appendChild(inner);

    for (const [key, v] of Object.entries(tree)) {
        if (key == "v") {
            buildPayload(inner, v);
        } else {
            const value: any = v;
            const iKey = Number.parseInt(key, 10);
            if (Number.isNaN(iKey)) {
                TODO("handle unexpected keys: " + key);
            }
            while (children.length <= iKey) {
                children.push(null);
            }
            const child = document.createElement("div");
            child.classList.add("tree-node");
            // if (typeof value.v?.level == "string")
            //     child.classList.add("tree-node-" + (<string>value.v.level).toLowerCase());

            buildTree(child, value, level + 1);

            children[iKey] = child;
        }
    }

    let i = 0;
    for (const child of children) {
        if (child == null)
            TODO();
        inner.appendChild(child);
        i++;
    }
}

function begin(on: any) {
    console.time("build tree");

    console.log(on);
    elById(HTMLElement, "select-wrapper").style.display = "none";

    removeFromParent(title);
    elById(HTMLElement, "view").style.display = "block";
    elById(HTMLElement, "view-header-right").appendChild(title);
    elById(HTMLElement, "view-header-msg").textContent = randomElement(msgs);

    const treeView = elById(HTMLDivElement, "tree-view");
    buildTree(treeView, on, 0);

    console.timeEnd("build tree");
}

function beginOnFile(file: File) {
    file.text().then(text => JSON.parse(text)).then(obj => begin(obj));
}

function openFileDialog() {
    const i = document.createElement("input");
    i.type = "file";
    i.accept = ".json";
    i.addEventListener("change", () => {
        if (i.files == null) return;
        const file = i.files[0];
        if (file == undefined) return;
        beginOnFile(file);
    });
    i.click();
}

window.addEventListener("drop", e => {
    // console.log(e);
    const dataTransfer = nn(e.dataTransfer);

    for (const file of dataTransfer.files) {
        e.preventDefault();
        beginOnFile(file);
        break;
    }
});
window.addEventListener("dragover", e => {
    e.preventDefault();
});

(<any>window).openFile = openFileDialog;
(<any>window).openDemo = () => {
    begin(demo);
};