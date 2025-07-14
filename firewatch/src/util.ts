import.meta.hot.accept;

export function nn<T>(value: T | null | undefined): T {
    if (value == null || value == undefined) {
        throw `value is ${value}`;
    } else {
        return value;
    }
}

export function intervalWhenShown(callback: () => void, interval: number) {
    let manager: number | null = null;

    function update() {
        if (document.hidden) {
            if (manager != null) {
                console.debug("removing interval");
                clearInterval(manager);
                manager = null;
            }
        } else {
            if (manager == null) {
                console.debug("setting interval");
                manager = setInterval(callback, interval);
            }
        }
    }

    document.addEventListener("visibilitychange", () => {
        update();
    });

    update();
}

export function elById<T>(ty: abstract new (...args: any[]) => T, id: string): T {
    const el = document.getElementById(id);
    if (el == null) {
        throw `element ${id} not found`;
    } else if (el instanceof ty) {
        return el;
    } else {
        throw `element ${id} does not have expected type`;
    }
}

export function removeFromParent(node: Node) {
    node.parentNode?.removeChild(node); // do nothing if node has no parent
}

export function TODO(message?: string): never {
    if (message != undefined) {
        throw "not yet implemented: " + message;
    } else {
        throw "not yet implemented";
    }
}

export function buildChild<K extends keyof HTMLElementTagNameMap>(parent: HTMLElement, name: K, func: (element: HTMLElementTagNameMap[K]) => void): HTMLElementTagNameMap[K] {
    const element = document.createElement(name);
    func(element);
    parent.appendChild(element);
    return element;
}

export function randomElement<T>(of: T[]): T {
    const val = of[Math.floor(Math.random() * of.length)];
    if (val == undefined)
        throw "array should not be empty";
    return val;
}