export interface Item {
    id: number,
    list_id: number,
    content: string,
}
export interface List {
    id: number,
    name: string,
}

export class Api {
    url: string;
    options: { headers: { [e: string]: string } };

    constructor(url: string, username: string, password: string) {
        this.url = url;
        this.options = {
            "headers": {
                "Authorization": "Basic " + btoa(username + ":" + password)
            }
        };
    }

    async get_lists(): Promise<List[]> {
        const res = await fetch(this.url + "/lists", this.options);
        return await res.json();
    }

    async get_list(list_id: number): Promise<List> {
        const res = await fetch(this.url + "/lists/" + list_id, this.options);
        return await res.json();
    }

    async get_all_items(): Promise<Item[]> {
        const res = await fetch(this.url + "/items", this.options);
        return await res.json();
    }

    async get_items(list_id: number): Promise<Item[]> {
        const res = await fetch(this.url + "/lists/" + list_id + "/items", this.options);
        return await res.json();
    }

    async get_item(item_id: number): Promise<Item> {
        const res = await fetch(this.url + "/items/" + item_id, this.options);
        return await res.json();
    }

    async create_list(name: string): Promise<List> {
        const res = await fetch(this.url + "/lists", {
            headers: {
                ...this.options.headers,
                "Content-Type": "application/json",
            },
            method: "POST",
            body: JSON.stringify({
                name,
            })
        });
        return await res.json();
    }

    async create_item(list_id: number, content: string): Promise<Item> {
        const res = await fetch(this.url + "/items", {
            headers: {
                ...this.options.headers,
                "Content-Type": "application/json",
            },
            method: "POST",
            body: JSON.stringify({
                list_id,
                content
            })
        });
        return await res.json();
    }

    async rename_list(list_id: number, new_name: string): Promise<void> {
        await fetch(this.url + "/lists/" + list_id, {
            headers: {
                ...this.options.headers,
                "Content-Type": "application/json",
            },
            method: "PATCH",
            body: JSON.stringify({
                name: new_name
            })
        });
    }

    async edit_item(item_id: number, new_content: string): Promise<void> {
        await fetch(this.url + "/items/" + item_id, {
            headers: {
                ...this.options.headers,
                "Content-Type": "application/json",
            },
            method: "PATCH",
            body: JSON.stringify({
                content: new_content
            })
        });
    }

    async delete_list(list_id: number): Promise<void> {
        await fetch(this.url + "/lists/" + list_id, {
            ...this.options,
            method: "DELETE"
        });
    }

    async delete_item(item_id: number): Promise<void> {
        await fetch(this.url + "/items/" + item_id, {
            ...this.options,
            method: "DELETE"
        })
    }
}
