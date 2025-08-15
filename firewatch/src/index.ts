import { title } from "./fireAnim";
import { buildChild, elById, nn, removeFromParent, TODO } from "./util";
import demo from "./demo.json";

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

class TreeStats {
    nodes: number = 0;
    htmlNodes: number = 0;

    buildEl<K extends keyof HTMLElementTagNameMap>(name: K, func?: (element: HTMLElementTagNameMap[K]) => void): HTMLElementTagNameMap[K] {
        this.htmlNodes++;
        const el = document.createElement(name);
        if (func != undefined)
            func(el);
        return el;
    };

    buildChild<K extends keyof HTMLElementTagNameMap>(parent: HTMLElement, name: K, func?: (element: HTMLElementTagNameMap[K]) => void): HTMLElementTagNameMap[K] {
        this.htmlNodes++;
        return buildChild(parent, name, func);
    }
}

function buildHeader(parent: HTMLElement, stats: TreeStats, innerElement: HTMLElement, payload: Payload) {
    stats.buildChild(parent, "span", el => {
        el.textContent = "[+]";
        el.classList.add("payload-toggle-collapsed");
        let collapsed = true;
        el.addEventListener("click", () => {
            if (collapsed) {
                innerElement.classList.remove("tree-node-inner-collapsed");
                el.textContent = "[-]";
            } else {
                innerElement.classList.add("tree-node-inner-collapsed");
                el.textContent = "[+]";
            }
            collapsed = !collapsed;
        });
    });
    if (payload.level != undefined) {
        const level = String(payload.level);
        const levelLo = level.toLowerCase();
        stats.buildChild(parent, "span", el => {
            el.textContent = level + " ";
            el.classList.add("payload-level");
            el.classList.add("payload-level-" + levelLo);
        });

        // info.classList.add("payload-info-" + levelLo);
        // parent.classList.add("tree-node-" + level.toLowerCase());
    }
    if (payload.name != undefined) {
        stats.buildChild(parent, "span", el => {
            el.textContent = String(payload.name);
            el.classList.add("payload-name");
        });
    }
}

function buildPayload(parent: HTMLElement, stats: TreeStats, payload: Payload) {
    const div = stats.buildEl("div");
    div.classList.add("payload");

    function addKvPair(special: boolean, key: string, value: string) {
        const pairEl = stats.buildEl("p");
        pairEl.classList.add("payload-pair");
        stats.buildChild(pairEl, "span", keyEl => {
            keyEl.textContent = key;
            keyEl.classList.add("payload-key");
            if (special)
                keyEl.classList.add("payload-key-special");
        }).insertAdjacentText("afterend", ": ");
        stats.buildChild(pairEl, "span", valEl => {
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

function buildTree(parent: HTMLElement, stats: TreeStats, tree: Tree, level: number) {
    parent.classList.add("tree-node-" + level % 2);
    let children: (HTMLElement | null)[] = [];

    const inner = stats.buildEl("div");
    if (tree.v != undefined) {
        const payload = tree.v;
        stats.buildChild(parent, "p", p => {
            p.classList.add("payload-info");
            buildHeader(p, stats, inner, payload);
        });
    }
    parent.appendChild(inner);

    inner.classList.add("tree-node-inner");
    // collapse all elements by default except for root
    if (level != 0)
        inner.classList.add("tree-node-inner-collapsed");

    for (const [key, v] of Object.entries(tree)) {
        if (key == "v") {
            buildPayload(inner, stats, v);
        } else {
            const value: any = v;
            const iKey = Number.parseInt(key, 10);
            if (Number.isNaN(iKey)) {
                TODO("handle unexpected keys: " + key);
            }
            while (children.length <= iKey) {
                children.push(null);
            }
            const child = stats.buildEl("div");
            child.classList.add("tree-node");
            // if (typeof value.v?.level == "string")
            //     child.classList.add("tree-node-" + (<string>value.v.level).toLowerCase());

            buildTree(child, stats, value, level + 1);

            children[iKey] = child;
            stats.nodes += 1;
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

    const treeView = elById(HTMLDivElement, "tree-view");
    for (const child of treeView.children) {
        if (child.id != "tree-view-root")
            removeFromParent(child);
    }
    const stats = new TreeStats();
    buildTree(treeView, stats, on, 0);

    console.timeEnd("build tree");
    elById(HTMLElement, "view-header-msg").textContent = `tree nodes: ${stats.nodes}; html nodes: ${stats.htmlNodes}`;
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