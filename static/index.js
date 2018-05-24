/*
 * 🌩 Place client
 * bodgy code below
 */

// main canvas
let canvas = document.querySelector('#canvas');
let ctx = canvas.getContext('2d');

// data canvas
let dCanvas = document.createElement('canvas');
let dCtx = dCanvas.getContext('2d');

// view offset
let offset = [0, 0];
// view scale
let scale = 1;

// scales the view
let scaleByFactor = function (factor, x, y) {
    scale *= factor;
    offset[0] = (offset[0] - x) * factor + x;
    offset[1] = (offset[1] - y) * factor + y;
}

// clamps the view parameters
let clampView = function () {
    let maxScale = Math.hypot(dCanvas.width, dCanvas.height) / 20;
    if (scale < 0.1) scaleByFactor(0.1 / scale, window.innerWidth / 2, window.innerHeight / 2);
    if (scale > maxScale) scaleByFactor(maxScale / scale, window.innerWidth / 2, window.innerHeight / 2);
    let padding = 20;
    let minX = 0;
    let minY = 0;
    let maxX = window.innerWidth - dCanvas.width * scale;
    let maxY = window.innerHeight - dCanvas.height * scale;
    if (maxX < minX) [minX, maxX] = [maxX, minX];
    if (maxY < minY) [minY, maxY] = [maxY, minY];
    offset[0] = Math.max(minX - padding, Math.min(maxX + padding, offset[0]));
    offset[1] = Math.max(minY - padding, Math.min(maxY + padding, offset[1]));
}

// cursor position on the screen
let cursorPos = [0, 0];

// current drawing color
let currentColor = [0, 0, 0];

// if true, will draw a cursor box on the canvas
let drawCursor = false;

// list of chat bubbles
let chatBubbles = [];

// list of broadcasts
let broadcasts = [];

// if true, is connected to the server
let isConnected = false;

let stopDrawLoop;

class Spring {
    constructor (force, damping) {
        this.x = 0;
        this.v = 0;
        this.value = 0;
        this.force = force;
        this.damping = damping;
    }

    static presetDefault (force) {
        return new this(force, Math.sqrt(force * 36 / 25));
    }

    static presetCritical (force) {
        return new this(force, 2 * Math.sqrt(force));
    }

    cforce () {
        return -this.force * (this.x - this.value) - this.damping * this.v;
    }

    update (dt) {
        if (dt > 0.33) dt = 0.33;
        let f = this.cforce();
        this.x += this.v * dt;
        this.v += f * dt;
    }
}

let lastDraw = Date.now();

// redraws main canvas
let redraw = function () {
    let dt = (-lastDraw + (lastDraw = Date.now())) / 1000;

    ctx.mozImageSmoothingEnabled = false;
    ctx.webkitImageSmoothingEnabled = false;
    ctx.msImageSmoothingEnabled = false;
    ctx.imageSmoothingEnabled = false;

    ctx.setTransform(window.devicePixelRatio, 0, 0, window.devicePixelRatio, 0, 0);

    ctx.fillStyle = '#000';
    ctx.fillRect(0, 0, window.innerWidth, window.innerHeight);

    if (!isConnected) ctx.globalAlpha = 0.5;

    ctx.translate(offset[0], offset[1]);
    ctx.scale(scale, scale);
    ctx.drawImage(dCanvas, 0, 0);

    let now = Date.now();

    ctx.save();
    ctx.scale(1 / scale, 1 / scale);
    ctx.translate(-offset[0], -offset[1]);
    let i = 0;
    let removeBroadcasts = [];
    for (let broadcast of broadcasts) {
        let bdt = (now - broadcast.time) / 1000
        let opacity = bdt > 10 ? 0 : bdt > 8 ? 1 - (bdt - 8) / 2 : 1;
        if (bdt > 10) removeBroadcasts.push(i);

        ctx.globalAlpha = opacity;
        ctx.fillStyle = '#fff';
        ctx.strokeStyle = '#000';
        ctx.lineWidth = 4;
        ctx.font = '24px sans-serif';
        let width = ctx.measureText(broadcast.data.text).width;
        let x = (window.innerWidth - width) / 2;
        let y = (window.innerHeight - 12) / 2;
        ctx.strokeText(broadcast.data.text, x, y);
        ctx.fillText(broadcast.data.text, x, y);

        i++;
    }

    let coffset = 0;
    for (let i of removeBroadcasts) {
        broadcasts.splice(i + (coffset--), 1);
    }
    ctx.restore();

    let cursorX = (cursorPos[0] - offset[0]) / scale;
    let cursorY = (cursorPos[1] - offset[1]) / scale;
    if (drawCursor) {
        ctx.lineWidth = 1 / scale;
        ctx.strokeStyle = `rgb(${currentColor[0]}, ${currentColor[1]}, ${currentColor[2]}`;
        ctx.strokeRect(cursorX | 0, cursorY | 0, 1, 1);
    }

    let removeBubbles = [];
    ctx.font = `12px sans-serif`;
    ctx.strokeStyle = '#000';
    ctx.lineWidth = 1 / scale;
    i = 0;
    let bubblePositions = [];
    for (let bubble of chatBubbles) {
        let bdt = (now - bubble.time) / 1000
        let opacity = bdt > 10 ? 0 : bdt > 8 ? 1 - (bdt - 8) / 2 : 1;
        let removeThis = false;

        let blx = bubble.data.x === null ? cursorX : bubble.data.x;
        let bly = bubble.data.y === null ? cursorY : bubble.data.y;

        if (bdt < 8) {
            bubble.scale.value = 1;
        } else if (bubble.scale.value === 1) {
            bubble.scale.value = 0;
            bubble.scale.v = 10;
        } else if (bubble.scale.x < 0) {
            removeThis = true;
        }
        bubble.scale.update(dt);

        let bx = 20 / scale;
        let by = -20 / scale;

        ctx.save();
        ctx.translate(blx, bly);
        ctx.scale(bubble.scale.x, bubble.scale.x);
        ctx.rotate((bubble.scale.x - 1) / 10);

        for (let pos of bubblePositions) {
            if (Math.abs(pos[0] - blx - bx) < 100 / scale && Math.abs(pos[1] - bly - by) < 15 / scale) {
                by += 20 / scale;
            }
        }

        bubble.offsetX.value = bx;
        bubble.offsetX.update(dt);
        bubble.offsetY.value = by;
        bubble.offsetY.update(dt);

        let dbx = bubble.offsetX.x;
        let dby = bubble.offsetY.x;

        ctx.globalAlpha = opacity;
        ctx.beginPath();
        ctx.moveTo(0, 0);
        ctx.lineTo(dbx, dby);
        ctx.stroke();
        let w = ctx.measureText(bubble.data.text).width;
        if (bubble.data.id_hue === null) {
            ctx.fillStyle = 'rgba(24, 131, 255, 0.7)';
        } else {
            ctx.fillStyle = `hsla(${bubble.data.id_hue * 360}, 70%, 40%, 0.7)`;
        }
        let rx = dbx;
        let ry = dby - 12 / scale;
        let rw = (20 + w) / scale;
        let rh = 18 / scale;
        let radius = rh / 2;
        ctx.beginPath();
        ctx.moveTo(rx + radius, ry);
        ctx.lineTo(rx + rw - radius, ry);
        ctx.arcTo(rx + rw, ry, rx + rw, ry + radius, radius);
        ctx.lineTo(rx + rw, ry + rh - radius);
        ctx.arcTo(rx + rw, ry + rh, rx + rw - radius, ry + rh, radius);
        ctx.lineTo(rx + radius, ry + rh);
        ctx.arcTo(rx, ry + rh, rx, ry + rh - radius, radius);
        ctx.lineTo(rx, ry + radius);
        ctx.arcTo(rx, ry, rx + radius, ry, radius);
        ctx.fill();
        if (bubble.data.is_admin) {
            ctx.strokeStyle = '#1883ff';
            ctx.stroke();
            ctx.strokeStyle = '#000';
        }
        ctx.fillStyle = '#fff';
        ctx.save();
        ctx.translate(dbx + (10 / scale), dby);
        ctx.scale(1 / scale, 1 / scale);
        ctx.fillText(bubble.data.text, 0, 0);
        ctx.restore();

        ctx.restore();

        bubblePositions.push([blx + bx, bly + by]);

        if (removeThis) removeBubbles.push(i);
        i += 1;
    }
    ctx.globalAlpha = 1;

    coffset = 0;
    for (let i of removeBubbles) {
        chatBubbles.splice(i + (coffset--), 1);
    }

    if (!chatBubbles.length && !broadcasts.length) stopDrawLoop();
};

let drawLoopID = 0;
let drawLoop = function drawLoop (id) {
    if (drawLoopID === id) requestAnimationFrame(() => drawLoop(id));
    redraw();
};

let startDrawLoop = function () {
    drawLoopID = Math.abs(drawLoopID) + 1;
    drawLoop(drawLoopID);
};
stopDrawLoop = function () {
    drawLoopID *= -1;
};

// resizes main canvas
let resizeCanvas = function () {
    canvas.style.width = window.innerWidth + 'px'
    canvas.style.height = window.innerHeight + 'px'
    canvas.width = window.innerWidth * window.devicePixelRatio
    canvas.height = window.innerHeight * window.devicePixelRatio
    clampView();
    redraw();
};
resizeCanvas();
window.addEventListener('resize', resizeCanvas);

// handles full update
let fullUpdate = function (w, h, data) {
    dCanvas.width = w;
    dCanvas.height = h;
    dCtx.clearRect(0, 0, w, h);

    let ibuf = Uint8ClampedArray.from(atob(data), x => x.charCodeAt(0));
    let idata = new ImageData(ibuf, w, h);
    dCtx.putImageData(idata, 0, 0);
    redraw()
};

// handles single pixel update
let drawPixel = function (x, y, r, g, b) {
    dCtx.fillStyle = `rgb(${r}, ${g}, ${b})`;
    dCtx.fillRect(x, y, 1, 1);
};

let drawRegion = function (x, y, w, h, data) {
    let ibuf = Uint8ClampedArray.from(atob(data), x => x.charCodeAt(0));
    let idata = new ImageData(ibuf, w, h);
    dCtx.putImageData(idata, x, y);
}

// if true, will log chat messages
let logChatMessages = false;

// websocket
let ws

let consoleWSDidOpen, consoleWSDidClose, consoleWsOnMessage;

// connects websocket
let init = function initWS () {
    let protocol = location.protocol === 'https:' ? 'wss://' : 'ws://';
    ws = new WebSocket(`${protocol}${location.host}${location.pathname}canvas`);
    ws.onopen = () => {
        isConnected = true;
        redraw();
        consoleWSDidOpen();
    };
    ws.onmessage = msg => {
        msg = JSON.parse(msg.data);
        if (msg.type === 'full-update') {
            fullUpdate(msg.data.w, msg.data.h, msg.data.data);
        } else if (msg.type === 'regions') {
            for (let region of msg.data) {
                drawRegion(region.x, region.y, region.w, region.h, region.data);
            }
            redraw()
        } else if (msg.type === 'chat-message') {
            if (logChatMessages) {
                console.info(`[CHAT] (${msg.data.x}, ${msg.data.y} h ${msg.data.id_hue}) ${msg.data.text}`);
            }
            chatBubbles.push({
                data: msg.data,
                time: Date.now(),
                scale: Spring.presetDefault(100),
                offsetX: Spring.presetCritical(100),
                offsetY: Spring.presetCritical(100),
            });
            startDrawLoop();
            redraw();
        } else if (msg.type === 'broadcast') {
            if (logChatMessages) {
                console.info(`[BROADCAST] ${msg.data.text}`);
            }

            broadcasts.push({
                data: msg.data,
                time: Date.now(),
            });

            startDrawLoop();
            redraw();
        } else if (msg.type === 'auth' || msg.type === 'console') {
            consoleWsOnMessage(msg);
        } else {
            console.log(msg);
        }
    };
    ws.onclose = () => {
        isConnected = false;
        setTimeout(init, 1000);
        redraw();
        consoleWSDidClose();
    };
}

canvas.addEventListener('mousemove', e => {
    cursorPos = [e.offsetX, e.offsetY];
});
canvas.addEventListener('mouseout', e => {
    drawCursor = false;
    redraw();
});

canvas.addEventListener('wheel', e => {
    e.preventDefault();
    if (e.ctrlKey) {
        let factor = (1 - e.deltaY / 100);
        scaleByFactor(factor, cursorPos[0], cursorPos[1]);
    } else {
        offset[0] -= e.deltaX;
        offset[1] -= e.deltaY;
    }
    clampView();
    redraw();
});

// color palette
let colors = [
    [0x00, 0x00, 0x00],
    [0xc9, 0x1b, 0x00],
    [0x00, 0xc2, 0x00],
    [0xc7, 0xc4, 0x00],
    [0x57, 0x75, 0xff],
    [0xca, 0x30, 0xc7],
    [0x00, 0xc5, 0xc7],
    [0xc7, 0xc7, 0xc7],
    [0x68, 0x68, 0x68],
    [0xff, 0x6e, 0x67],
    [0x5f, 0xfa, 0x68],
    [0xff, 0xfc, 0x67],
    [0x9c, 0xa2, 0xff],
    [0xff, 0x77, 0xff],
    [0x60, 0xfd, 0xff],
    [0xff, 0xff, 0xff]
];
currentColor = colors[0];
let updateColorDisp;

// sets a pixel to the current color
let setPixel = function (x, y) {
    ws.send(JSON.stringify({
        type: 'set-pixel',
        data: {
            x: x | 0,
            y: y | 0,
            r: currentColor[0],
            g: currentColor[1],
            b: currentColor[2]
        }
    }));
}

// gets snapshot of all data pixels
let getPixels = function () {
    let iData = dCtx.getImageData(0, 0, dCanvas.width, dCanvas.height);

    return {
        imageData: iData,
        getPixel: function (x, y) {
            x |= 0;
            y |= 0;
            if (x < 0 || y < 0 || x >= this.imageData.width || y >= this.imageData.height) return undefined;
            let i = (this.imageData.width * y + x) * 4;
            return [
                this.imageData.data[i],
                this.imageData.data[i + 1],
                this.imageData.data[i + 2]
            ];
        }
    };
};

let downPos = null;
let prevPos = null;
let downOffset = null;
let moveDistance = 0;
canvas.addEventListener('mousedown', e => {
    downPos = [e.offsetX, e.offsetY];
    prevPos = downPos.slice();
    downOffset = [offset[0], offset[1]];
});
let mouseClickTolerance = 4
canvas.addEventListener('mousemove', e => {
    if (downPos) {
        moveDistance += Math.hypot(e.offsetX - prevPos[0], e.offsetY - prevPos[1]);
        prevPos = [e.offsetX, e.offsetY];
        if (moveDistance >= mouseClickTolerance) {
            offset[0] = (e.offsetX - downPos[0]) + downOffset[0];
            offset[1] = (e.offsetY - downPos[1]) + downOffset[1];
            clampView();
        } else if (mouseClickTolerance === Infinity && !(e.altKey || e.ctrlKey)) {
            let x = (e.offsetX - offset[0]) / scale;
            let y = (e.offsetY - offset[1]) / scale;
            setPixel(x, y);
        }
    }
    drawCursor = true;
    redraw();
});
let downScale = 1;
let pinchDownPos = null;
let downCenter = null;
canvas.addEventListener('touchstart', e => {
    drawCursor = false;
    e.preventDefault();
    downPos = [e.touches[0].clientX, e.touches[0].clientY];
    prevPos = downPos.slice();
    downOffset = [offset[0], offset[1]];
    if (e.touches[1]) {
        pinchDownPos = [e.touches[1].clientX, e.touches[1].clientY];
        downCenter = [
            (e.touches[1].clientX - downPos[0]) / 2 + downPos[0],
            (e.touches[1].clientY - downPos[1]) / 2 + downPos[1]
        ];
        downScale = scale;
    }
});
canvas.addEventListener('touchmove', e => {
    if (downPos) {
        let x = e.touches[0].clientX;
        let y = e.touches[0].clientY;
        moveDistance += Math.hypot(x - prevPos[0], y - prevPos[1]);
        prevPos = [x, y];
        if (pinchDownPos && e.touches[1]) {
            let startDiag = Math.hypot(pinchDownPos[0] - downPos[0], pinchDownPos[1] - downPos[1]);
            let curDiag = Math.hypot(e.touches[1].clientX - x, e.touches[1].clientY - y);
            let factor = curDiag / startDiag;
            scale = downScale * factor;
            offset[0] = (downOffset[0] - downCenter[0]) * factor + downCenter[0];
            offset[1] = (downOffset[1] - downCenter[1]) * factor + downCenter[1];
        } else if (!pinchDownPos) {
            offset[0] = (x - downPos[0]) + downOffset[0];
            offset[1] = (y - downPos[1]) + downOffset[1];
        }
        clampView();
        redraw();
    }
});

canvas.addEventListener('mouseup', e => {
    if (moveDistance < mouseClickTolerance) {
        let x = (e.offsetX - offset[0]) / scale;
        let y = (e.offsetY - offset[1]) / scale;
        if (e.altKey || e.ctrlKey) {
            currentColor = getPixels().getPixel(x, y) || currentColor;
            updateColorDisp();
        } else {
            setPixel(x, y);
        }
        offset = downOffset || offset;
    }
    downPos = null;
    downOffset = null;
    moveDistance = 0;
});

canvas.addEventListener('touchend', e => {
    if (moveDistance < 4 && !pinchDownPos) {
        let x = (prevPos[0] - offset[0]) / scale;
        let y = (prevPos[1] - offset[1]) / scale;
        setPixel(x, y);
    }
    downPos = null;
    downOffset = null;
    pinchDownPos = null;
    moveDistance = 0;
});

{
    // controls

    let controls = document.createElement('div');
    controls.id = 'controls';
    document.body.appendChild(controls);

    let colorDisp = document.createElement('div');
    colorDisp.className = 'current-color-disp';
    let hexI;
    controls.appendChild(colorDisp);
    updateColorDisp = function () {
        let color = currentColor;
        colorDisp.style.background = `rgb(${color[0]}, ${color[1]}, ${color[2]})`;
        let pad = x => (x = x.toString(16)).length === 2 ? x : '0' + x;
        hexI.value = pad(color[0]) + pad(color[1]) + pad(color[2]);
    };

    for (let color of colors) {
        let btn = document.createElement('button');
        btn.className = 'color-btn';
        controls.appendChild(btn);
        btn.style.background = `rgb(${color[0]}, ${color[1]}, ${color[2]})`;
        btn.addEventListener('click', e => {
            currentColor = color;
            updateColorDisp();
        });
    }

    let zoomIn = document.createElement('button');
    let zoomOut = document.createElement('button');
    zoomIn.className = 'zoom-btn zoom-in';
    zoomOut.className = 'zoom-btn zoom-out';
    controls.appendChild(zoomIn);
    controls.appendChild(zoomOut);
    zoomIn.textContent = '+';
    zoomOut.textContent = '-';
    zoomIn.addEventListener('click', () => {
        scaleByFactor(1.25, window.innerWidth / 2, window.innerHeight / 2);
        clampView();
        redraw();
    });
    zoomOut.addEventListener('click', () => {
        scaleByFactor(1 / 1.25, window.innerWidth / 2, window.innerHeight / 2);
        clampView();
        redraw();
    });

    let hexIW = document.createElement('div');
    controls.appendChild(hexIW);
    hexIW.className = 'hex-input-wrap';
    hexIW.textContent = '#';
    hexI = document.createElement('input');
    hexIW.appendChild(hexI);

    let applyHexI = () => {
        let v = hexI.value;
        v = v.substr(0, 6);
        if (v.length < 6) {
            v = '000000'.substr(-5 + v.length) + v;
        }
        let r = parseInt(v.substr(0, 2), 16);
        let g = parseInt(v.substr(2, 2), 16);
        let b = parseInt(v.substr(4, 2), 16);
        currentColor = [r, g, b];
        updateColorDisp();
    };

    hexI.addEventListener('keydown', e => {
        if (e.key === 'Enter') applyHexI();
    });
    hexI.addEventListener('blur', applyHexI);

    updateColorDisp();

    let helpBtn = document.createElement('button');
    helpBtn.className = 'help-btn';
    helpBtn.textContent = '?';
    controls.appendChild(helpBtn);
    helpBtn.addEventListener('click', e => {
        let help = document.querySelector('#help');
        if (help.classList.contains('open')) help.classList.remove('open');
        else help.classList.add('open');
    });
}

window.addEventListener('keyup', e => {
    if (document.activeElement.tagName === 'INPUT') return;
    if (e.key === 't') {
        let input = document.createElement('input');
        input.className = 'chat-input';
        input.placeholder = 'Chat';
        Object.assign(input.style, {
            top: cursorPos[1] + 'px',
            left: cursorPos[0] + 'px',
        });
        input.addEventListener('keydown', e => {
            if (e.key === 'Enter') {
                ws.send(JSON.stringify({
                    type: 'chat-message',
                    data: {
                        x: (cursorPos[0] - offset[0]) / scale,
                        y: (cursorPos[1] - offset[1]) / scale,
                        text: input.value
                    }
                }));
                input.blur();
            } else if (e.key === 'Escape') {
                input.blur();
            }
        });
        input.addEventListener('blur', e => {
            input.parentNode.removeChild(input);
        });
        document.body.appendChild(input);
        input.focus();
    }
});

{
    // console
    let panel = document.querySelector('#console');
    let messages = document.querySelector('#console-messages');
    let prompt = document.querySelector('#console-input-prompt');
    let input = document.querySelector('#console-input');
    input.disabled = true;

    window.addEventListener('keyup', e => {
        if (e.key === 'F4') {
            e.preventDefault();
            if (panel.classList.contains('open')) {
                panel.classList.remove('open');
                input.blur();
            } else {
                panel.classList.add('open');
                input.focus();
            }
        }
    });

    let state = 'login';

    let setPrompt = (text, isPassword) => {
        prompt.textContent = text;
        if (!isPassword && input.type === 'password') {
            input.value = '';
        }
        input.type = isPassword ? 'password' : 'text';
    };

    consoleWSDidOpen = () => {
        input.disabled = false;
        setPrompt('login');
        state = 'login';
    };
    consoleWSDidClose = () => {
        setPrompt('Disconnected');
        input.disabled = true;
    };

    let login;

    let putMessage = (msg, classNames = '') => {
        let message = document.createElement('div');
        message.className = 'console-message ' + classNames;
        message.textContent = msg;
        let scrollHeight = messages.scrollHeight;
        messages.appendChild(message);

        if (scrollHeight - messages.scrollTop === messages.offsetHeight) {
            messages.scrollTop = messages.scrollHeight - messages.offsetHeight;
        }
    };

    input.addEventListener('keydown', e => {
        if (e.key === 'Enter') {
            let cmd = input.value;
            input.value = '';

            if (state === 'login') {
                login = cmd;
                state = 'password';
                setPrompt('password', true);
            } else if (state === 'password') {
                ws.send(JSON.stringify({
                    type: 'auth',
                    data: {
                        login,
                        password: cmd,
                    }
                }));
                state = 'waiting';
                setPrompt('logging in…');
            } else if (state === 'waiting') {
                input.value = cmd;
            } else if (state === 'logged-in') {
                putMessage(cmd, 'command');
                ws.send(JSON.stringify({
                    type: 'console',
                    data: cmd,
                }));
            }
        }
    });

    consoleWsOnMessage = msg => {
        if (msg.type === 'auth') {
            if (msg.data) {
                setPrompt('place');
                state = 'logged-in';
            } else {
                state = 'login';
                if (msg.data === null) {
                        setPrompt('waiting');
                        setTimeout(() => {
                            setPrompt('login');
                        }, 3000);
                } else {
                    setPrompt('login');
                }
            }
        } else if (msg.type === 'console') {
            putMessage(msg.data);
        }
    };
}

init();
