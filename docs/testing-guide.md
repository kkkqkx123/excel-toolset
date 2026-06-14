# 测试数据准备指南

本文档提供创建测试数据的方法，用于验证新增功能。

## 测试文件创建

### 使用 Excel 创建测试文件

#### 测试文件 1: formula_test.xlsx

**Sheet1 - 基础公式**

|       | A        | B       | C        | D        | E       |
|-------|----------|---------|----------|----------|---------|
| 1     | Product  | Price   | Quantity | Total    | Tax     |
| 2     | A        | 100     | 10       | =B2*C2   | =D2*0.1 |
| 3     | B        | 200     | 5        | =B3*C3   | =D3*0.1 |
| 4     | C        | 150     | 8        | =B4*C4   | =D4*0.1 |
| 5     |         |         |          | =SUM(D2:D4) | =SUM(E2:E4) |
| 6     | Average  | =AVERAGE(B2:B4) | =AVERAGE(C2:C4) | =AVERAGE(D2:D4) | =AVERAGE(E2:E4) |
| 7     | Max      | =MAX(B2:B4) | =MAX(C2:C4) | =MAX(D2:D4) | =MAX(E2:E4) |
| 8     | Min      | =MIN(B2:B4) | =MIN(C2:C4) | =MIN(D2:D4) | =MIN(E2:E4) |
| 9     | Count    | =COUNT(B2:B4) | =COUNT(C2:C4) | =COUNT(D2:D4) | =COUNT(E2:E4) |

**Sheet2 - 复杂公式**

|       | A        | B       | C        | D        | E       |
|-------|----------|---------|----------|----------|---------|
| 1     | Quarter  | Q1      | Q2       | Q3       | Q4      |
| 2     | Sales    | 1000    | 1500     | 2000     | 2500    |
| 3     | Growth   |         | =(B3-B2)/B2 | =(C3-B3)/B3 | =(D3-C3)/C3 |
| 4     | Total    | =SUM(B2:E2) | | | |
| 5     | Target   | 5000    | | | |
| 6     | Status   | =IF(B4>=B5,"达标","未达标") | | | |

#### 测试文件 2: search_test.xlsx

**Sheet1 - 混合数据**

|       | A        | B       | C        | D        | E       |
|-------|----------|---------|----------|----------|---------|
| 1     | ID       | Name    | Value    | Status   | Code    |
| 2     | 1        | Alice   | 1000     | Active   | A001    |
| 3     | 2        | Bob     | 2000     | Inactive | B002    |
| 4     | 3        | Charlie | 1500     | Active   | C003    |
| 5     | 4        | David   | 3000     | Inactive | D004    |
| 6     | 5        | Eve     | 2500     | Active   | E005    |

**Sheet2 - 公式数据**

|       | A        | B       | C        | D        | E       |
|-------|----------|---------|----------|----------|---------|
| 1     | Item     | Quantity| Price    | Total    | Note    |
| 2     | Item1    | 10      | =SUM(100,200) | =B2*C2 | High value |
| 3     | Item2    | 5       | 150      | =B3*C3 | Normal  |
| 4     | Item3    | 8       | =AVERAGE(100,200) | =B4*C4 | Low value|

#### 测试文件 3: comments_test.xlsx

**Sheet1 - 带批注的数据**

|       | A        | B       | C        | D        |
|-------|----------|---------|----------|----------|
| 1     | ID       | Name    | Value    | Status   |
| 2     | 1        | Alice   | 1000     | Active   |
| 3     | 2        | Bob     | 2000     | Inactive |

**添加批注**:
- B2: "关键用户，VIP客户"
- C2: "年度销售额"
- B3: "新用户，需要跟进"

#### 测试文件 4: named_ranges_test.xlsx

**Sheet1 - 销售数据**

|       | A        | B       | C        | D        |
|-------|----------|---------|----------|----------|
| 1     | Month    | Sales   | Cost     | Profit   |
| 2     | Jan      | 10000   | 6000     | =B2-C2   |
| 3     | Feb      | 12000   | 7000     | =B3-C3   |
| 4     | Mar      | 15000   | 8000     | =B4-C4   |
| 5     | Total    | =SUM(B2:B4) | =SUM(C2:C4) | =SUM(D2:D4) |

**创建命名范围**:
- `SalesData`: B2:B4
- `CostData`: C2:C4
- `ProfitData`: D2:D4
- `TotalSales`: B5
- `TotalCost`: C5
- `TotalProfit`: D5

#### 测试文件 5: conditional_format_test.xlsx

**Sheet1 - 绩效数据**

|       | A        | B       | C        | D        |
|-------|----------|---------|----------|----------|
| 1     | Employee | Target  | Actual   | Percent  |
| 2     | Alice    | 100     | 120      | =C2/B2*100|
| 3     | Bob      | 100     | 95       | =C3/B3*100|
| 4     | Charlie  | 100     | 110      | =C4/B4*100|
| 5     | David    | 100     | 80       | =C5/B5*100|
| 6     | Eve      | 100     | 105      | =C6/B6*100|

**条件格式设置**:
- D2:D6: >100 为红色加粗
- D2:D6: <90 为黄色
- D2:D6: 90-100 为绿色

## Python 脚本创建测试文件

如果你有 Python 和 `openpyxl` 库，可以使用以下脚本自动创建测试文件：

```python
import openpyxl
from openpyxl.styles import PatternFill, Font, Border
from openpyxl.comments import Comment

def create_formula_test():
    wb = openpyxl.Workbook()

    # Sheet1 - 基础公式
    ws1 = wb.active
    ws1.title = "Sheet1"

    # Headers
    ws1['A1'] = "Product"
    ws1['B1'] = "Price"
    ws1['C1'] = "Quantity"
    ws1['D1'] = "Total"
    ws1['E1'] = "Tax"

    # Data rows
    data = [
        ["A", 100, 10],
        ["B", 200, 5],
        ["C", 150, 8],
    ]

    for i, row in enumerate(data, start=2):
        ws1[f'A{i}'] = row[0]
        ws1[f'B{i}'] = row[1]
        ws1[f'C{i}'] = row[2]
        ws1[f'D{i}'] = f"=B{i}*C{i}"
        ws1[f'E{i}'] = f"=D{i}*0.1"

    # Summary rows
    ws1['D5'] = "=SUM(D2:D4)"
    ws1['E5'] = "=SUM(E2:E4)"

    ws1['A6'] = "Average"
    ws1['B6'] = "=AVERAGE(B2:B4)"
    ws1['C6'] = "=AVERAGE(C2:C4)"
    ws1['D6'] = "=AVERAGE(D2:D4)"
    ws1['E6'] = "=AVERAGE(E2:E4)"

    ws1['A7'] = "Max"
    ws1['B7'] = "=MAX(B2:B4)"
    ws1['C7'] = "=MAX(C2:C4)"
    ws1['D7'] = "=MAX(D2:D4)"
    ws1['E7'] = "=MAX(E2:E4)"

    ws1['A8'] = "Min"
    ws1['B8'] = "=MIN(B2:B4)"
    ws1['C8'] = "=MIN(C2:C4)"
    ws1['D8'] = "=MIN(D2:D4)"
    ws1['E8'] = "=MIN(E2:E4)"

    wb.save("formula_test.xlsx")
    print("Created formula_test.xlsx")

def create_search_test():
    wb = openpyxl.Workbook()

    # Sheet1 - Mixed data
    ws1 = wb.active
    ws1.title = "Sheet1"

    ws1['A1'] = "ID"
    ws1['B1'] = "Name"
    ws1['C1'] = "Value"
    ws1['D1'] = "Status"
    ws1['E1'] = "Code"

    data = [
        [1, "Alice", 1000, "Active", "A001"],
        [2, "Bob", 2000, "Inactive", "B002"],
        [3, "Charlie", 1500, "Active", "C003"],
        [4, "David", 3000, "Inactive", "D004"],
        [5, "Eve", 2500, "Active", "E005"],
    ]

    for i, row in enumerate(data, start=2):
        ws1[f'A{i}'] = row[0]
        ws1[f'B{i}'] = row[1]
        ws1[f'C{i}'] = row[2]
        ws1[f'D{i}'] = row[3]
        ws1[f'E{i}'] = row[4]

    # Sheet2 - Formula data
    ws2 = wb.create_sheet("Sheet2")

    ws2['A1'] = "Item"
    ws2['B1'] = "Quantity"
    ws2['C1'] = "Price"
    ws2['D1'] = "Total"
    ws2['E1'] = "Note"

    ws2['A2'] = "Item1"
    ws2['B2'] = 10
    ws2['C2'] = "=SUM(100,200)"
    ws2['D2'] = "=B2*C2"
    ws2['E2'] = "High value"

    ws2['A3'] = "Item2"
    ws2['B3'] = 5
    ws2['C3'] = 150
    ws2['D3'] = "=B3*C3"
    ws2['E3'] = "Normal"

    ws2['A4'] = "Item3"
    ws2['B4'] = 8
    ws2['C4'] = "=AVERAGE(100,200)"
    ws2['D4'] = "=B4*C4"
    ws2['E4'] = "Low value"

    wb.save("search_test.xlsx")
    print("Created search_test.xlsx")

def create_comments_test():
    wb = openpyxl.Workbook()

    ws1 = wb.active
    ws1.title = "Sheet1"

    ws1['A1'] = "ID"
    ws1['B1'] = "Name"
    ws1['C1'] = "Value"
    ws1['D1'] = "Status"

    ws1['A2'] = 1
    ws1['B2'] = "Alice"
    ws1['C2'] = 1000
    ws1['D2'] = "Active"

    ws1['A3'] = 2
    ws1['B3'] = "Bob"
    ws1['C3'] = 2000
    ws1['D3'] = "Inactive"

    # Add comments
    comment1 = Comment("关键用户，VIP客户", "Admin")
    ws1['B2'].comment = comment1

    comment2 = Comment("年度销售额", "Admin")
    ws1['C2'].comment = comment2

    comment3 = Comment("新用户，需要跟进", "Admin")
    ws1['B3'].comment = comment3

    wb.save("comments_test.xlsx")
    print("Created comments_test.xlsx")

def create_named_ranges_test():
    wb = openpyxl.Workbook()

    ws1 = wb.active
    ws1.title = "Sheet1"

    ws1['A1'] = "Month"
    ws1['B1'] = "Sales"
    ws1['C1'] = "Cost"
    ws1['D1'] = "Profit"

    data = [
        ["Jan", 10000, 6000],
        ["Feb", 12000, 7000],
        ["Mar", 15000, 8000],
    ]

    for i, row in enumerate(data, start=2):
        ws1[f'A{i}'] = row[0]
        ws1[f'B{i}'] = row[1]
        ws1[f'C{i}'] = row[2]
        ws1[f'D{i}'] = f"=B{i}-C{i}"

    ws1['B5'] = "=SUM(B2:B4)"
    ws1['C5'] = "=SUM(C2:C4)"
    ws1['D5'] = "=SUM(D2:D4)"

    # Create named ranges
    wb.create_named_range("SalesData", ws1, "B2:B4")
    wb.create_named_range("CostData", ws1, "C2:C4")
    wb.create_named_range("ProfitData", ws1, "D2:D4")
    wb.create_named_range("TotalSales", ws1, "B5")
    wb.create_named_range("TotalCost", ws1, "C5")
    wb.create_named_range("TotalProfit", ws1, "D5")

    wb.save("named_ranges_test.xlsx")
    print("Created named_ranges_test.xlsx")

def create_conditional_format_test():
    wb = openpyxl.Workbook()

    ws1 = wb.active
    ws1.title = "Sheet1"

    ws1['A1'] = "Employee"
    ws1['B1'] = "Target"
    ws1['C1'] = "Actual"
    ws1['D1'] = "Percent"

    data = [
        ["Alice", 100, 120],
        ["Bob", 100, 95],
        ["Charlie", 100, 110],
        ["David", 100, 80],
        ["Eve", 100, 105],
    ]

    for i, row in enumerate(data, start=2):
        ws1[f'A{i}'] = row[0]
        ws1[f'B{i}'] = row[1]
        ws1[f'C{i}'] = row[2]
        ws1[f'D{i}'] = f"=C{i}/B{i}*100"

    # Conditional formatting
    red_fill = PatternFill(start_color="FF0000", end_color="FF0000", fill_type="solid")
    red_font = Font(color="FFFFFF", bold=True)

    yellow_fill = PatternFill(start_color="FFFF00", end_color="FFFF00", fill_type="solid")

    green_fill = PatternFill(start_color="00FF00", end_color="00FF00", fill_type="solid")

    for row in range(2, 7):
        cell = ws1[f'D{row}']
        cell.conditional_formatting.add(
            f"D{row}",
            openpyxl.formatting.rule.CellIsRule(operator='greaterThan', formula=['100'], fill=red_fill, font=red_font)
        )
        cell.conditional_formatting.add(
            f"D{row}",
            openpyxl.formatting.rule.CellIsRule(operator='lessThan', formula=['90'], fill=yellow_fill)
        )
        cell.conditional_formatting.add(
            f"D{row}",
            openpyxl.formatting.rule.CellIsRule(operator='between', formula=['90', '100'], fill=green_fill)
        )

    wb.save("conditional_format_test.xlsx")
    print("Created conditional_format_test.xlsx")

if __name__ == "__main__":
    create_formula_test()
    create_search_test()
    create_comments_test()
    create_named_ranges_test()
    create_conditional_format_test()
    print("All test files created successfully!")
```

## 测试用例

### 1. 公式分析测试

```bash
# 测试依赖追踪
curl -X POST http://localhost:3000/api/formula/trace_dependencies \
  -H "Content-Type: application/json" \
  -d '{
    "path": "formula_test.xlsx",
    "sheet": "Sheet1",
    "cell": "D5"
  }'

# 测试公式解释（中文）
curl -X POST http://localhost:3000/api/formula/explain \
  -H "Content-Type: application/json" \
  -d '{
    "path": "formula_test.xlsx",
    "sheet": "Sheet1",
    "cell": "D5",
    "language": "zh"
  }'

# 测试逻辑分析
curl -X POST http://localhost:3000/api/formula/explain_logic \
  -H "Content-Type: application/json" \
  -d '{
    "path": "formula_test.xlsx",
    "sheet": "Sheet1",
    "cell": "D5",
    "language": "zh"
  }'
```

### 2. 搜索测试

```bash
# 搜索值
curl -X POST http://localhost:3000/api/search/workbook \
  -H "Content-Type: application/json" \
  -d '{
    "path": "search_test.xlsx",
    "pattern": "1000",
    "search_type": "value",
    "match_type": "exact"
  }'

# 搜索公式
curl -X POST http://localhost:3000/api/search/workbook \
  -H "Content-Type: application/json" \
  -d '{
    "path": "search_test.xlsx",
    "pattern": "SUM",
    "search_type": "formula",
    "match_type": "contains"
  }'

# 正则表达式搜索
curl -X POST http://localhost:3000/api/search/workbook \
  -H "Content-Type: application/json" \
  -d '{
    "path": "search_test.xlsx",
    "pattern": "^\\d+$",
    "search_type": "value",
    "match_type": "regex"
  }'
```

### 3. 批注测试

```bash
# 读取批注
curl -X POST http://localhost:3000/api/comments/get \
  -H "Content-Type: application/json" \
  -d '{
    "path": "comments_test.xlsx",
    "sheet": "Sheet1",
    "cell": "B2"
  }'

# 添加批注
curl -X POST http://localhost:3000/api/comments/add \
  -H "Content-Type: application/json" \
  -d '{
    "path": "comments_test.xlsx",
    "sheet": "Sheet1",
    "cell": "C3",
    "comment": "待审核数据"
  }'
```

### 4. 命名范围测试

```bash
# 列出命名范围
curl -X GET http://localhost:3000/api/named_ranges/list/named_ranges_test.xlsx

# 获取命名范围值
curl -X POST http://localhost:3000/api/named_ranges/get_value \
  -H "Content-Type: application/json" \
  -d '{
    "path": "named_ranges_test.xlsx",
    "name": "SalesData"
  }'
```

### 5. 条件格式测试

```bash
# 添加条件格式
curl -X POST http://localhost:3000/api/conditional_format/add \
  -H "Content-Type: application/json" \
  -d '{
    "path": "conditional_format_test.xlsx",
    "sheet": "Sheet1",
    "range": "D2:D6",
    "rule_type": "cellvalue",
    "condition": ">100",
    "format": {
      "font_color": "FFFFFF",
      "bold": true,
      "background_color": "FF0000"
    }
  }'
```

## 预期结果

### 公式分析
- 依赖追踪应返回正确的前驱和后继单元格
- 公式解释应返回清晰的中文/英文描述
- 逻辑分析应返回详细的计算步骤

### 搜索
- 应返回所有匹配的单元格
- 应包含上下文信息
- 正则表达式应正确匹配

### 批注
- 应正确读取批注内容
- 添加批注应成功
- 修改和删除批注应正常工作

### 命名范围
- 应列出所有命名范围
- 应正确返回范围值
- 创建和删除应成功

### 条件格式
- 应成功添加条件格式
- 删除应正确清除格式

## 注意事项

1. 确保文件路径正确
2. 确保服务器正在运行
3. 使用 dry_run 参数预览操作
4. 备份重要文件测试
5. 检查权限设置

## 故障排除

### 编译错误
- 检查 Rust 工具链安装
- 检查依赖版本兼容性
- 清理缓存：`cargo clean`

### 运行时错误
- 检查文件是否存在
- 检查文件格式是否正确
- 检查权限设置

### API 错误
- 检查请求格式
- 检查参数有效性
- 查看错误消息

---

**准备就绪**: 运行测试前确保所有功能已编译并通过基本测试