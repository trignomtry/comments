let ws;
let commentInput;
const panelChild = document.createElement("div");


function setGlobal(key, value) {
    chrome.storage.local.set({ [key]: value });
}

function getGlobal(key, callback) {
    chrome.storage.local.get([key], (result) => {
        console.log("Gotten!");
        console.log(result[key]);
        callback(result[key]);
    });
}
async function run(appKey) {

    // Create toggle button
    const btn = document.createElement('button');
    btn.id = 'slideToggleBtn';
    btn.textContent = '☰';
    document.documentElement.appendChild(btn);

    // Create slide-out panel
    const panel = document.createElement('div');
    panel.style.backgroundColor = "#313131";
    panel.style.position = "fixed";
    panel.style.top = "0";
    panel.style.right = "-420px";
    panel.style.width = "400px";
    panel.style.height = "100vh";
    panel.style.zIndex = "9999";
    panel.style.transition = "right 0.3s ease";
    panel.id = 'universalCommentsslidePanel';
    document.documentElement.appendChild(panel);
    panel.innerHTML = `<form id="commentForm">
                        <button id="emojiButtonUC" type="button">😀</button>
                        <input type="text" id="commentInput" placeholder="Write a comment..."  />
                        <button type="submit">Post</button>
                       </form>`;
    document.getElementById("commentForm").addEventListener("submit", function (e) {
        e.preventDefault();
        comment();
    });
    commentInput = document.getElementById("commentInput");
    let commentbt = document.getElementById("emojiButtonUC");

    // Emoji picker implementation
    const emojiPicker = document.createElement("div");
    emojiPicker.id = "emojiPickerUC";
    emojiPicker.style.position = "absolute";
    emojiPicker.style.bottom = "60px";
    emojiPicker.style.right = "10px";
    emojiPicker.style.background = "#222";
    emojiPicker.style.border = "1px solid #555";
    emojiPicker.style.padding = "5px";
    emojiPicker.style.display = "none";
    emojiPicker.style.borderRadius = "5px";
    emojiPicker.style.zIndex = "10000";

    const emojis = ["😀", "😂", "😍", "😎", "😭", "👍"];
    emojis.forEach(emoji => {
        const btn = document.createElement("button");
        btn.textContent = emoji;
        btn.style.fontSize = "20px";
        btn.style.margin = "3px";
        btn.style.cursor = "pointer";
        btn.onclick = () => {
            commentInput.value += emoji;
            emojiPicker.style.display = "none";
            pickerVisible = false;
            commentInput.focus();
        };
        emojiPicker.appendChild(btn);
    });

    document.body.appendChild(emojiPicker);

    let pickerVisible = false;
    commentbt.onclick = function () {
        pickerVisible = !pickerVisible;
        emojiPicker.style.display = pickerVisible ? "block" : "none";
    };
    panel.insertBefore(panelChild, panel.firstChild);

    let isOpen = false;
    btn.addEventListener('click', () => {
        isOpen = !isOpen;
        panel.style.right = isOpen ? '0' : '-420px';
    });

    try {
        initWS(appKey);

        if (!appKey) {
            console.log("Not signed in");
            panelChild.innerHTML = "<h1>Please sign in to continue</h1>";
            let bt = document.createElement("button");
            bt.innerText = "Sign in with Vortice";
            bt.onclick = function () {
                window.open("https://comments.vortice.app/auth/");
                window.addEventListener("message", function (event) {
                    if (event.data.kind == "app_key") {
                        console.log(event.data);
                        setGlobal("appKey", event.data.key);
                        panelChild.innerHTML = "";

                        initWS(event.data.key);
                    }
                });
            };
            panelChild.appendChild(bt);
            return;
        }

        initWS(appKey);


    } catch (e) {
        console.error("Fetch failed:", e);
        panel.innerHTML = "Couldn't load comments. (Network error)";
    }

}

getGlobal("appKey", (appKey) => {
    run(appKey);
});



function comment() {
    ws.send(JSON.stringify({ "text": commentInput.value }));
    commentInput.value = "";
}

function initWS(key) {
    let fullPath = window.location.host + window.location.pathname;
    if (window.location.host.includes("youtube.com")) {
        fullPath += window.location.search + window.location.hash;
    }
    ws = new WebSocket("https://comments.vortice.app/api/" + key + "/comments/" +
        encodeURIComponent(fullPath));
    ws.onmessage = function (event) {
        let commen = JSON.parse(event.data);
        console.log("comments: ", commen);

        if (commen.error) {
            panel.innerHTML = "Couldn't load comments: " + commen.error;
            return;
        }
        if (commen.length === 0) {
            panelChild.innerText = "No comments on this page. Be the first to comment!";
        }
        commen.forEach(element => {
            let commentDiv = document.createElement("div");
            commentDiv.classList.add("commentDiv");
            commentDiv.innerHTML = "<p class='usernameText'>" + element.username + "</p><p class='commentText'>" + element.text + "</p>";
            panelChild.appendChild(commentDiv);
        });
    }
    ws.onclose = function () {
        console.log("closed");
    }
    ws.onerror = function () {
        setGlobal("appKey", null);
    }
}