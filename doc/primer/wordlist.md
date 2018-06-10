# 字典

TODO

Forth 能執行 `+` 、 `-` 、`*` 、 `/` 這些指令，因為它的系統內有一部字典 (wordlist).。
可以使用 `words` 檢查 Forth 字典內有哪些字。試一下：

```
rf> words

(abort) quit evaluate accept 1/sec hz rpm um/msec mm/min usec msec sec minute
hr rad deg um mm meter fnegate fceil fround floor fmax fmin f< f0= f0< f~ f**
f/ f* f- f+ f>s s>f fover frot fnip fswap fdup fdrop fsqrt fatan2 fatan facos
fasin fsincos ftan fcos fsin fabs f@ f! fvariable fconstant utime ntime max-u
max-n .s words flush f. . .( ." s" type emit interpret-token compile-token
token-empty? compiling? bye abort reset handle-error error? handler! marker ,
] ' create variable constant : parse char parse-word between max min negate abs
mod / 2over 2swap 2drop 2dup rot <> > 0<> 0> 0= not false true +loop loop ?do
do recurse again repeat while begin then else if ; [char] [ \ ( base c! c@
allot here chars char+ ! @ cells cell+ /mod * + - 1- 1+ rshift lshift xor or
and invert < = 0< ?stacks depth nip over swap drop dup execute 2r@ 2r> 2>r r@
r> >r j i leave unloop _+loop _loop _qdo _do 0branch branch _s" flit lit halt
exit noop  ok
```

字典如何記載一個指令？

| 名稱 | 行為        | 資料來源 |
|-----|-------------|-----|
| `+`  | 執行整數加法 | 來自資料堆疊 |
| `f+` | 執行浮數點加法 | 來自浮點堆疊  |

-----------
## 常數、變數

| 名稱 | 行為        | 資料資料 |
|-----|-------------|-----|
| `true`  | 將 -1 放上堆疊 | 來自資料空間 |
| `false` | 將 0 放上堆疊 | 來自資料空間  |

### 本節指令集
| 指令 | 堆疊效果           | 說明                        | 口語唸法 |
|-----|-------------------|-----------------------------|--------|
| `constant` | ( -- )|  | constant |
| `variable` | ( -- )|  | variable |
| `fconstant` | ( -- )|  | f-constant |
| `fvariable` | ( -- )|  | f-variable |

-----------
## 本章指令集

| 指令 | 堆疊效果           | 說明                        | 口語唸法 |
|-----|-------------------|-----------------------------|--------|
| `words` | ( -- )|  | words |
| `marker` | ( -- )|  | marker |
| `empty` | ( -- )|  | empty |
| `:` | ( -- )|  | colon |
| `;` | ( -- )|  | semicolon |
| `constant` | ( -- )|  | constant |
| `variable` | ( -- )|  | variable |
| `fconstant` | ( -- )|  | f-constant |
| `fvariable` | ( -- )|  | f-variable |
| `create` | ( -- )|  | create |
| `cells` | ( -- )|  | cells |
| `cell+` | ( -- )|  | cell+ |
| `align` | ( -- )|  | align |
| `aligned` | ( -- )|  | aligned |
| `allot` | ( -- )|  | allot |
| `here` | ( -- )|  | here |
| `,` | ( -- )|  | comma |
| `does>` | ( -- )|  | does |
| `@` | ( -- )|  | fetch |
| `!` | ( n a -- )| 將 n 存在位址 a  | store |
| `2@` | ( -- )| | two-fetch |
| `2!` | ( -- )| | two-store |
| `+!` | ( n a -- )| 將位址 a 內的整數加 n | plus-store |
| `f@` | ( -- )|  | f-fetch |
| `f!` | ( -- )|  | f-store |