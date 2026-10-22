```bash { window=bob }
echo "$SIGNING $PACE"
```

```bash { window=bob if=SIGNING:external }
echo external
```

```bash { window=bob if=SIGNING:internal }
echo internal
```

```bash { window=bob if=PACE:slow }
echo slow
```

```bash { window=bob if=PACE:fast status=19 }
echo fast
```
