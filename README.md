# contrast

 - 正则表达式对比小程序，输入电子签及纸质签，回车即可对输入内容正则表达式处理后进行对比，并发出不同的提示音。
 - 当正则表达式处理后的内容包含特殊符号，则自动过滤这些符号。

# rules.toml

 - 正则表达式配置文件，支持多个规则，规则名称为sn_rules和paper_rules，规则内容使用正则表达式；如出现规则冲突则以匹配成功第一个为准，如规则为空或未匹配成功则对比原内容。

 ```toml
[sn_rules]
rule1 = "sn=([A-Z0-9]+)"

[paper_rules]
rule1 = "sn=([A-Z0-9]+)"
rule2 = ",(.*)"
```
