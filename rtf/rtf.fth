-work

: quit
    reset
    begin receive ."  " evaluate-input
    compiling? not if ."  ok"  then
    13 emit flush-output
    again ;

: (abort)
    0stacks error -2 1 within not if
      .token space .error
      source-id dup if dup
        ."  (" .source-path
        ." :"  load-line# @  0 .r ." : " .source-line ." )"
      else drop
      then
      ." , " .backtrace  13 emit
    then flush-output 0error quit ;

\ Cold start
: cold
    2 halt  3 halt  4 halt  5 halt
    ['] (abort) handler!
    evaluate-input quit ;

marker -work
