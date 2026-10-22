```bash { window=bob }
echo ready > note
```

```bash { window=bob mode=observe }
test -s note && echo seen
```

```bash { window=bob }
rm note
```
