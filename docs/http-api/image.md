## 图片/形状

### POST /api/image/insert
插入图片。

**请求体**：
```json
{
  "path": "data.xlsx",
  "config": {
    "sheet": "Sheet1",
    "image_path": "logo.png",
    "anchor_cell": "B2",
    "width": 200,
    "height": 100
  }
}
```

### POST /api/image/remove
移除图片。

**请求体**：
```json
{
  "path": "data.xlsx",
  "sheet": "Sheet1",
  "anchor_cell": "B2"
}
```

### POST /api/image/shape/insert
插入形状（矩形、椭圆、线条）。

**请求体**：
```json
{
  "path": "data.xlsx",
  "config": {
    "sheet": "Sheet1",
    "shape_type": "rectangle",
    "anchor_cell": "D5",
    "width": 100,
    "height": 50
  }
}
```
