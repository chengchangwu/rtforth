# 整數運算

如上一章所述執行除錯版的 rtForth： 

```
$ ./target/debug/examples/rf
```

或執行最佳化版本的 rtForth： 

```
$ ./target/release/examples/rf
```

在 `rf>` 提示後輸入 `2 17 + .` 後按 Enter， 

```
rf> 2 17 + .
19  ok
rf> 
```
這兒發生了什麼事？首先 Forth 是直譯式語言，內建[直譯器](https://zh.wikipedia.org/wiki/%E7%9B%B4%E8%AD%AF%E5%99%A8) (英文：interpreter) 。當執行 `rf` 時，rtForth 會起動文本直譯器 (text interpreter) ，先印出 `rf>`，等待使用者輸入，再一個字 (word) 一個字的，從輸入緩衝區 (input buffer) 掃描 (scan) 使用者的輸入，在字典 (word list) 中查詢字的定義 (definition) 並執行。完成後顯示 `ok`，告訴使用者執行成功。若失敗則印出錯誤訊息。然後再印出提示字串 (prompt) `rf>` 請使用者繼續輸入。

```
文本直譯器 (Text interpreter)

      Input buffer
        2     1  7     +     .
      +--+--+--+--+--+--+--+--+--
      |50|32|49|55|32|43|32|46|
      +--+--+--+--+--+--+--+--+--
                    |
                    v
                +-------+ yes     +---------+
                | Word? |-------->| Execute |----+
                +-------+         +---------+    |
                    | no                         |
                    v             +---------+    |
                +---------+ yes   | Number  |    |
                | Number? |------>|   on    |    |
                +---------+       | stack   |    |
                    | no          +---------+    |
                    v                  |         |
                +---------------+      |         |
                | Error message |      |         |
                +---------------+      |         |
                    |                  |         |
      Output buffer v                  v         |
      +---+---+-----------        +---------+    |
      |111|101|           <-------| ok      |<---+
      +---+---+-----------        +---------+
         o   k
```

Forth 的字和字是以空白或是換行符號隔開的。因此 `2 17 + .` 裡面一共有四個字。`2` 和 `17` 是數字。而 `+` 和 `.` 是已經在字典中定義好的指令 (instruction) 。

當直譯器無法在字典中找到這個字，比如 `2` 時，會將這個字轉換成數字 (number)，存放在某記憶體中。這塊記憶體被稱為資料堆疊 (data stack)。稱為堆疊的原因是，這些轉成數字的資料會依次序排列，新來的被堆在舊的上面。就像下圖：

```

輸入緩衝區 (Input buffer) 內存放的數字是 2 17 + . 的 ASCII 碼。
      +--+--+--+--+--+--+--+--+--
      |50|32|49|55|32|43|32|46|
      +--+--+--+--+--+--+--+--+--
        2     1  7     +     .

文本直譯器處理了 2 17 後的結果：

資料堆疊 (Data stack)
           |    |
           +----+
           | 17 | 疊頂(Top)
           +----+
           |  2 |
           +----+
```
也可以以左右方式呈現資料堆疊，

```
資料堆疊 (Data stack)
           +---+----+--
           | 2 | 17 |
           +---+----+--
                疊頂
                (Top)
```

或是為了方便，在文件中採用 `( 2 17 )` 這樣的註解呈現。

在上述例子中，`+` 從疊頂拿了兩個整數，`2` 和 `17`，相加後將結果 `19` 放回疊頂。所以堆疊的變化如下：

```
指令 + 的效果：

Data stack |    |
           +----+
           | 17 |    +     |    |
           +----+   ===>   +----+
           |  2 |          | 19 |
           +----+          +----+
```

在文件中會以 `( 2 17 -- 19 )` 的註解方式呈現堆疊的效果。

最後的指令 `.` 從疊頂拿走了一個整數，將它轉成文字放在輸出緩衝區中。最後輸出緩衝區內的文字被顯示在螢幕上。

```
指令 . 的效果：

資料堆疊 (Data stack)

           |    |    .
           +----+   ===>
           | 19 |          |    |
           +----+          +----+

輸出緩衝區 (Output buffer) 內放的是 19 的 ASCII 碼
    +--+--+--
    |49|57|
    +--+--+--
      1  9
```

### 本節指令集

| 指令 | 堆疊效果及指令說明                        | 口語唸法 |
|-----|----------------------------------------|--------|
| `+` | ( n1 n2 -- sum ) &emsp; 將資料堆疊上的最後的兩個整數 n1 n2 相加，將結果 sum 放回堆疊 | plus   |
| `.` | ( n -- ) &emsp; 印出資料堆疊上最後的整數 n，並將它從堆疊上移除 | dot |
| `(` | ( -- ) &emsp; 註解，因為是指令，之後必須接一個空白。會忽略這空白之後一直到下一個右括弧 ) 之間的文字 | paren |

---------------
## 更多的整數四則運算

Forth 提供了許多整數運算的指令，要執行每個指令前，必須先把所需的資料先放進資料堆疊。這種先資料再指令的方式，被稱為後位法。而我們熟悉的四則運算的表示法則被稱為中位法。下表比較了中位法和後位法：

| 運算  | 中位法 | 後位法 |
|------|-------|-------|
| 加    |1 + 2 | 1 2 + |
| 減    |3 - 4 | 3 4 - |
| 乘    |5 * 6 | 5 6 * |
| 求商  |7 / 8 | 7 8 / |
| 求餘數|7 mod 8 | 7 8 mod |
| 求商數和餘數 | 不存在表示法 | 7 8 /mod |

實測以上運算如下：

```
rf> 1 2 + .  3 4 - .  5 6 * .  7 8 / .  7 8 mod .
3 -1 30 0 7  ok
```

在計算時，特意將 `1 2 + .` 和 `3 4 - .` 以二個空格隔開，這有點像是英文把句子分成片語，片語再分成單字。寫 Forth 的人常將幾個
Forth 指令構成的片語和片語用兩個或三個空格隔開，使得程式更容易閱讀。

`/mod` 指令在堆疊上留下兩個數字，餘數和商。因此需要兩個 `.` 將它們印出來。測試如下：

```
rf> 7 8 /mod . .
0 7  ok
```
注意在堆疊上的次序是 `( 餘數 商 )`。因此，第一個 `.` 會印出商，第二個會印出餘數。

### 本節指令集

| 指令 | 堆疊效果及指令說明                        | 口語唸法 |
|-----|----------------------------------------|--------|
| `-` | ( n1 n2 -- diff ) &emsp; 將 n1 減去 n2。diff 是 n1, n2 的差 | minus  |
| `*` | ( n1 n2 -- prod ) &emsp; 將 n1 乘以 n2。prod 是 n1, n2 的乘積 | star |
| `/` | ( n1 n2 -- quot ) &emsp; 將 n1 除以 n2。quot 是 n1 除以 n2 後的商數 | slash |
| `mod` | ( n1 n2 -- rem ) &emsp; 將 n1 除以 n2。rem 是 n1 除以 n2 後的餘數 | mod |
| `/mod` | ( n1 n2 -- rem quot ) &emsp; 將 n1 除以 n2。rem 是 n1 除以 n2 後的餘數，quot 是商數 | slash-mod |

------------------------
## 整數函式

Forth 提供了數學計算上常見的函式：

| 運算        | 函式     | 後位法 |
|------------|---------|-------|
| 求絕對值     | abs(-5) | -5 abs |
| 求加法反元素 | -(6)     | 6 negate |
| 最小值      | min(7,8) | 7 8 min |
| 最大值      | max(9,0) | 9 0 max |

處理複雜的數學計算時，中位法依賴括弧決定指令執行的優先次序。後位法不使用括弧。由堆疊上資料的次序及及指令執行的次序決定。
以下以例子說明：

例一： 中位法 2 &times; (3 + 4) 和 (3 + 4) &times; 2
```
rf> 3 4 + 2 * .
14  ok
```
中位法的 2 &times; (3 + 4) 和 (3 + 4) &times; 2 都是先算 3 + 4，再乘以 2。所以後位法都是 `3 4 + 2 *` 。 

例二：中位法 (2 &times; 3 - 4) / 5 + 6
```
rf> 2 3 * 4 - 5 / 6 + .
6  ok
```
注意這兒的除法 `/` 是整數除法，求的是整數的商數。如果使用浮點數運算，會得到 6.4。浮點數運算請見下一章。

例三：中位法 abs(2 - 6) / min(2, 6)，也就是 2 - 6 的絕對值除以 2 和 6 的最小值。
```
rf> 2 6 - abs 2 6 min / .
2  ok
```

例四：中位法 -(3 + 4)<sup>2</sup>。
```
rf> 3 4 + 3 4 + * negate .
-49  ok
```
在此 `3 4 +` 計算了兩次，在未來章節會討論如何以程式簡化平方、立方的計算問題。

### 本節指令集

| 指令 | 堆疊效果及指令說明                        | 口語唸法 |
|-----|----------------------------------------|--------|
| `abs` | ( n -- u ) &emsp; u 是 n 的絕對值  | abs |
| `negate` | ( n1 -- n2 ) &emsp; n2 是 n1 的加法反元素， n1+n2=0 | negate |
| `min` | ( n1 n2 -- n3 ) &emsp; n3 是 n1 和 n2 中較小的數 | min |
| `max` | ( n1 n2 -- n3 ) &emsp; n3 是 n1 和 n2 中較大的數 | max |

-------------
## 本章重點整理

* 文本直譯器 (text interpreter)
* 資料堆疊 (data stack)
* 堆疊效果 (stack effect)：FORTH 文件常以 `( before -- after )` 的方式呈現執行指令時堆疊的變化。在 `--` 前的是指令執行前疊頂的內容。在 `--` 後的是執行後疊頂的內容。
* 字 (word)
* 指令 (instruction)
* 字典或指令集  (word list)
* 定義 (definition)
* 輸入緩衝區 (input buffer)
* 掃描 (scan)
* 輸出緩衝區 (output buffer)
* 整數 (integer)

-------------------------------------
## 本章指令集

| 指令 | 堆疊效果           | 說明                        | 口語唸法 |
|-----|-------------------|-----------------------------|--------|
| `.` | ( n -- ) &emsp; 印出資料堆疊上最後的整數，並將它從堆疊上移除 | dot |
| `(` | ( -- ) &emsp; 註解，因為是指令，之後必須接一個空白。會忽略這空白之後一直到下一個右括弧 ) 之間的文字 | paren |
| `+` | ( n1 n2 -- n1+n2 ) &emsp; 將資料堆疊上的最後的兩個整數相加，結果放回堆疊 | plus   |
| `-` | ( n1 n2 -- diff ) &emsp; 將 n1 減去 n2。diff 是 n1, n2 的差 | minus  |
| `*` | ( n1 n2 -- prod ) &emsp; 將 n1 乘以 n2。prod 是 n1, n2 的乘積 | star |
| `/` | ( n1 n2 -- quot ) &emsp; 將 n1 除以 n2。quot 是 n1 除以 n2 後的商數 | slash |
| `mod` | ( n1 n2 -- rem ) &emsp; 將 n1 除以 n2。rem 是 n1 除以 n2 後的餘數 | mod |
| `/mod` | ( n1 n2 -- rem quot ) &emsp; 將 n1 除以 n2。rem 是 n1 除以 n2 後的餘數，quot 是商數 | slash-mod |
| `abs` | ( n -- u ) &emsp; u 是 n 的絕對值  | a-b-s |
| `negate` | ( n1 -- n2 ) &emsp; n2 是 n1 的加法反元素， n1+n2=0 | negate |
| `min` | ( n1 n2 -- n3 ) &emsp; n3 是 n1 和 n2 中較小的數 | min |
| `max` | ( n1 n2 -- n3 ) &emsp; n3 是 n1 和 n2 中較大的數 | max |
