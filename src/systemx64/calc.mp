<!DOCTYPE html>
<html lang="ru">
<head>
<meta charset="UTF-8">
<meta name="window-size" content="334x426">
<title>Калькулятор</title>
<style>
body{margin:0;font-family:Arial,sans-serif;background:#f6f6f6;}
.stage{position:relative;width:240px;height:264px;margin:0 auto;}
.stage > *{position:absolute;margin:0;}
</style>
</head>
<body>
<div class="stage">
<div id="d" style="left:0px;top:0px;width:230px;height:30px;padding:4px;border:1px solid #9a9a9a;background:#ffffff;text-align:right;font-size:20px;">0</div>
<button type="button" onclick="press('C')" style="left:0px;top:44px;width:56px;height:36px;">C</button>
<button type="button" onclick="press('<')" style="left:60px;top:44px;width:56px;height:36px;">&larr;</button>
<button type="button" onclick="press('/')" style="left:120px;top:44px;width:56px;height:36px;">&divide;</button>
<button type="button" onclick="press('*')" style="left:180px;top:44px;width:56px;height:36px;">&times;</button>
<button type="button" onclick="press('7')" style="left:0px;top:88px;width:56px;height:36px;">7</button>
<button type="button" onclick="press('8')" style="left:60px;top:88px;width:56px;height:36px;">8</button>
<button type="button" onclick="press('9')" style="left:120px;top:88px;width:56px;height:36px;">9</button>
<button type="button" onclick="press('-')" style="left:180px;top:88px;width:56px;height:36px;">-</button>
<button type="button" onclick="press('4')" style="left:0px;top:132px;width:56px;height:36px;">4</button>
<button type="button" onclick="press('5')" style="left:60px;top:132px;width:56px;height:36px;">5</button>
<button type="button" onclick="press('6')" style="left:120px;top:132px;width:56px;height:36px;">6</button>
<button type="button" onclick="press('+')" style="left:180px;top:132px;width:56px;height:36px;">+</button>
<button type="button" onclick="press('1')" style="left:0px;top:176px;width:56px;height:36px;">1</button>
<button type="button" onclick="press('2')" style="left:60px;top:176px;width:56px;height:36px;">2</button>
<button type="button" onclick="press('3')" style="left:120px;top:176px;width:56px;height:36px;">3</button>
<button type="button" onclick="press('=')" style="left:180px;top:176px;width:56px;height:36px;">=</button>
<button type="button" onclick="press('0')" style="left:0px;top:220px;width:116px;height:36px;">0</button>
<button type="button" onclick="press('.')" style="left:120px;top:220px;width:56px;height:36px;">.</button>
</div>
<script>
var cur = "0", acc = 0, op = "", fresh = true;

function show(t) { document.getElementById("d").textContent = t; }

function calc(a, b, o) {
  if (o == "+") return a + b;
  if (o == "-") return a - b;
  if (o == "*") return a * b;
  if (o == "/") return b == 0 ? null : a / b;
  return b;
}

function fmt(x) { return String(Math.round(x * 10000000000) / 10000000000); }

function reset() { cur = "0"; acc = 0; op = ""; fresh = true; }

function press(k) {
  if (cur == "Ошибка") { reset(); }
  if (k == "C") {
    reset();
  } else if (k == "<") {
    cur = cur.length > 1 ? cur.slice(0, cur.length - 1) : "0";
  } else if (k == "+" || k == "-" || k == "*" || k == "/" || k == "=") {
    var v = parseFloat(cur);
    if (op != "" && !fresh) {
      var r = calc(acc, v, op);
      if (r === null) { reset(); cur = "Ошибка"; show(cur); return; }
      acc = r;
    } else {
      acc = v;
    }
    cur = fmt(acc);
    op = (k == "=") ? "" : k;
    fresh = true;
  } else {
    if (fresh) {
      cur = (k == ".") ? "0." : k;
      fresh = false;
    } else if (k == ".") {
      if (cur.indexOf(".") < 0) { cur = cur + "."; }
    } else {
      cur = (cur == "0") ? k : cur + k;
    }
  }
  show(cur);
}
</script>
</body>
</html>