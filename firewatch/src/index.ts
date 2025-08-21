import { title } from "./fireAnim";
import { buildChild, elById, nn, removeFromParent, TODO } from "./util";
import demo from "./demo.json";

interface Payload {
    level?: unknown;
    name?: unknown;
    ctx?: any;
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

interface NodeKv {
    name: string,
    special: boolean,
    value: string;
}

function prepareNodeMap(payload: Payload | undefined): NodeKv[] {
    const kvs: NodeKv[] = [];
    if (payload == undefined)
        return kvs;
    if (payload.file != undefined && payload.line != undefined)
        kvs.push({
            name: "at",
            special: true,
            value: `${payload.file}:${payload.line}`
        });

    const ctx = payload.ctx;
    if (ctx != undefined)
        for (const [key, value] of Object.entries(ctx)) {
            // the message is already inserted into the header for non-spans
            if (payload.is_span === false && key == "message")
                continue;

            kvs.push({
                name: key,
                value: typeof (value) == "string" ? value : JSON.stringify(value),
                special: false
            });
        }

    return kvs;
}

function buildHeader(parent: HTMLElement, stats: TreeStats, innerElement: HTMLElement, payload: Payload, isMapEmpty: boolean) {
    stats.buildChild(parent, "span", span => {
        span.classList.add("payload-toggle-collapsed");
        if (isMapEmpty) {
            span.classList.add("payload-toggle-collapsed-empty");
            // stats.buildChild(span, "pre", pre => {
            //     pre.textContent = "   ";
            // });
            return;
        }

        span.classList.add("payload-toggle-collapsed-interactive");
        span.textContent = "[+]";
        let collapsed = true;
        span.addEventListener("click", () => {
            if (collapsed) {
                innerElement.classList.remove("tree-node-inner-collapsed");
                span.textContent = "[-]";
            } else {
                innerElement.classList.add("tree-node-inner-collapsed");
                span.textContent = "[+]";
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
    const putName = (name: any) => stats.buildChild(parent, "span", el => {
        el.textContent = String(name);
        el.classList.add("payload-name");
    });
    if (payload.is_span === false && payload.ctx?.message != null) {
        putName(payload.ctx.message);
    } else if (payload.name != undefined) {
        putName(payload.name);
    }
}

function buildNodeMap(parent: HTMLElement, stats: TreeStats, map: NodeKv[]) {
    const div = stats.buildEl("div");
    div.classList.add("payload");

    for (const entry of map) {
        const pairEl = stats.buildEl("p");
        pairEl.classList.add("payload-pair");
        stats.buildChild(pairEl, "span", key => {
            key.textContent = entry.name;
            key.classList.add("payload-key");
            if (entry.special)
                key.classList.add("payload-key-special");
        }).insertAdjacentText("afterend", ": ");
        stats.buildChild(pairEl, "span", val => {
            val.textContent = entry.value;
            val.classList.add("payload-val");
        });

        div.appendChild(pairEl);
    }
    parent.appendChild(div);
}

function buildTree(parent: HTMLElement, stats: TreeStats, tree: Tree, level: number) {
    const map = prepareNodeMap(tree.v);
    const payload = tree.v;

    parent.classList.add("tree-node-" + level % 2);
    let children: (HTMLElement | null)[] = [];

    const inner = stats.buildEl("div");
    if (payload != null) {
        stats.buildChild(parent, "p", p => {
            p.classList.add("payload-info");
            buildHeader(p, stats, inner, payload, map.length == 0);
        });
    }
    parent.appendChild(inner);

    inner.classList.add("tree-node-inner");
    // collapse all elements by default except for root
    if (level != 0)
        inner.classList.add("tree-node-inner-collapsed");

    buildNodeMap(inner, stats, map);

    for (const [key, v] of Object.entries(tree)) {
        if (key == "v") {
            continue;
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