"""
Ultimate Budget Tracker v3 — ultimate_budget_v3.xlsx
Premium Etsy-style: KPI cards, donut/bar charts, mini calendar, spending tracker.
"""

import calendar
from openpyxl import Workbook
from openpyxl.styles import Font, PatternFill, Alignment, Border, Side
from openpyxl.utils import get_column_letter
from openpyxl.worksheet.datavalidation import DataValidation
from openpyxl.chart import BarChart, DoughnutChart, LineChart, Reference
from openpyxl.chart.series import SeriesLabel
from openpyxl.chart.label import DataLabel

# ─── PALETTE ────────────────────────────────────────────────────────────────
SAGE_DARK   = "3D5C3A"
SAGE_MID    = "5C8A52"
SAGE_LIGHT  = "C8DDB8"
SAGE_PALE   = "EEF4E8"
TERRA       = "B05E3A"
TERRA_LIGHT = "EDD5C5"
CREAM       = "FAF7F2"
GOLD        = "BFA06A"
WHITE       = "FFFFFF"
CHARCOAL    = "2A2A2A"
INPUT_BLUE  = "1A4FA0"
GREEN_OK    = "1E8449"
RED_ALERT   = "B03A2E"
BORDER_GRAY = "DCDCDC"

FMT_CURR = '#,##0;[RED]-#,##0;"-"'
FMT_PCT  = '0.0%'
FMT_DATE = 'MM/DD/YYYY'

MONTHS      = ["Jan","Feb","Mar","Apr","May","Jun","Jul","Aug","Sep","Oct","Nov","Dec"]
MONTH_NAMES = ["January","February","March","April","May","June",
               "July","August","September","October","November","December"]

EXPENSE_CATS = [
    ("HOUSING",               ["Rent/Mortgage","Utilities","Electricity","Internet","Home Insurance"]),
    ("TRANSPORT",             ["Car Payment","Fuel","Car Insurance","Parking","Public Transport"]),
    ("FOOD & DINING",         ["Groceries","Restaurants","Coffee","Food Delivery","Work Lunch"]),
    ("HEALTH & WELLNESS",     ["Health Insurance","Gym","Pharmacy","Doctor","Dental"]),
    ("LIFESTYLE",             ["Streaming Services","Entertainment","Hobbies","Personal Care","Books"]),
    ("PERSONAL",              ["Clothing","Gifts","Education","Pet Care","Miscellaneous"]),
    ("SAVINGS & INVESTMENTS", ["Emergency Fund","Pension/IRA","Stocks","Vacation Fund","Other Savings"]),
    ("DEBT PAYMENTS",         ["Credit Card 1","Credit Card 2","Student Loan","Car Loan","Other"]),
]

# ─── STYLE HELPERS ──────────────────────────────────────────────────────────
def fl(h): return PatternFill("solid", fgColor=h)
def fn(size=9, bold=False, color=CHARCOAL, italic=False):
    return Font(name="Calibri", size=size, bold=bold, color=color, italic=italic)
def al(h="left", v="center", wrap=False):
    return Alignment(horizontal=h, vertical=v, wrap_text=wrap)

THIN_GRAY = Side(style="thin",   color=BORDER_GRAY)
MED_GOLD  = Side(style="medium", color=GOLD)
MED_SAGE  = Side(style="medium", color=SAGE_MID)
NO_SIDE   = Side(style=None)

def tborder(): return Border(left=THIN_GRAY,right=THIN_GRAY,top=THIN_GRAY,bottom=THIN_GRAY)
def gold_bottom(): return Border(left=THIN_GRAY,right=THIN_GRAY,top=THIN_GRAY,bottom=MED_GOLD)
def med_border(): return Border(left=MED_SAGE,right=MED_SAGE,top=MED_SAGE,bottom=MED_SAGE)
def bot_med(): return Border(left=THIN_GRAY,right=THIN_GRAY,top=THIN_GRAY,
                              bottom=Side(style="medium",color=SAGE_MID))

def no_gridlines(ws): ws.sheet_view.showGridLines = False
def freeze(ws, cell): ws.freeze_panes = cell
def set_widths(ws, d):
    for col, w in d.items(): ws.column_dimensions[col].width = w

def set_cell(ws, row, col, value="", bold=False, size=9, color=CHARCOAL,
             bg=WHITE, halign="left", fmt=None, italic=False, height=None):
    c = ws.cell(row=row, column=col, value=value)
    c.font      = fn(size, bold, color, italic)
    c.fill      = fl(bg)
    c.alignment = al(halign, "center")
    c.border    = tborder()
    if fmt: c.number_format = fmt
    if height: ws.row_dimensions[row].height = height
    return c

def merge_set(ws, r1, c1, r2, c2, value="", bold=False, size=9, color=CHARCOAL,
              bg=WHITE, halign="center", fmt=None, italic=False,
              border_fn=None, height=None):
    ws.merge_cells(start_row=r1,start_column=c1,end_row=r2,end_column=c2)
    c = ws.cell(row=r1, column=c1, value=value)
    c.font      = fn(size, bold, color, italic)
    c.fill      = fl(bg)
    c.alignment = al(halign, "center", wrap=True)
    c.border    = (border_fn or tborder)()
    if fmt: c.number_format = fmt
    if height: ws.row_dimensions[r1].height = height
    return c

def inp(ws, row, col, value=0, fmt=FMT_CURR, bg=WHITE):
    return set_cell(ws,row,col,value,False,9,INPUT_BLUE,bg,"right",fmt)

def frm(ws, row, col, formula, fmt=FMT_CURR, color=CHARCOAL, bg=WHITE):
    return set_cell(ws,row,col,formula,False,9,color,bg,"right",fmt)

# ─── CHART HELPERS ──────────────────────────────────────────────────────────
def style_chart(chart, title=""):
    chart.style   = 10
    chart.title   = title
    if hasattr(chart, 'legend') and chart.legend:
        chart.legend.position = "b"
    return chart

def add_chart(ws, chart, anchor):
    ws.add_chart(chart, anchor)

# ═══════════════════════════════════════════════════════════════════════════
# MONTHLY SHEET BUILDER  — returns row_tracker dict
# ═══════════════════════════════════════════════════════════════════════════
def build_month(ws, month_name, month_short):
    no_gridlines(ws)
    ws.sheet_properties.tabColor = SAGE_MID
    NCOLS = 7

    # Row 1-2: titles
    merge_set(ws,1,1,1,NCOLS, f"{month_name.upper()} · MONTHLY BUDGET",
              True,18,WHITE,SAGE_DARK,"center",height=48)
    merge_set(ws,2,1,2,NCOLS,
              f"Track your income and expenses for {month_name}",
              False,9,WHITE,GOLD,"left",italic=True,height=20)

    # Row 3: col headers
    for i,h in enumerate(["CATEGORY","SUBCATEGORY","BUDGETED","ACTUAL","DIFFERENCE","% USED","NOTES"]):
        set_cell(ws,3,i+1,h,True,9,WHITE,SAGE_DARK,"center",height=20)

    rt = {}   # row tracker
    r  = 4

    # ── INCOME ──────────────────────────────────────────────────────────────
    merge_set(ws,r,1,r,NCOLS,"INCOME",True,10,WHITE,TERRA,"left",height=22); r+=1
    inc_start = r
    for j,item in enumerate(["Primary Income","Side Income","Other Income"]):
        bg = TERRA_LIGHT if j%2 else CREAM
        set_cell(ws,r,1,"INCOME",bg=bg,height=18)
        set_cell(ws,r,2,item,bg=bg)
        inp(ws,r,3,0,FMT_CURR,bg); inp(ws,r,4,0,FMT_CURR,bg)
        frm(ws,r,5,f"=IFERROR(C{r}-D{r},0)",FMT_CURR,CHARCOAL,bg)
        frm(ws,r,6,f"=IFERROR(D{r}/C{r},0)",FMT_PCT,CHARCOAL,bg)
        set_cell(ws,r,7,"",bg=bg); r+=1
    inc_end = r-1

    ti = r
    merge_set(ws,ti,1,ti,2,"TOTAL INCOME",True,9,CHARCOAL,SAGE_LIGHT,"left",
              border_fn=bot_med,height=20)
    for col,f in [(3,f"=SUM(C{inc_start}:C{inc_end})"),(4,f"=SUM(D{inc_start}:D{inc_end})")]:
        fc=ws.cell(row=ti,column=col,value=f); fc.font=fn(9,True,CHARCOAL)
        fc.fill=fl(SAGE_LIGHT); fc.alignment=al("right"); fc.border=bot_med()
        fc.number_format=FMT_CURR
    frm(ws,ti,5,f"=IFERROR(C{ti}-D{ti},0)",FMT_CURR,CHARCOAL,SAGE_LIGHT)
    frm(ws,ti,6,f"=IFERROR(D{ti}/C{ti},0)",FMT_PCT,CHARCOAL,SAGE_LIGHT)
    set_cell(ws,ti,7,"",bg=SAGE_LIGHT)
    rt["ti"] = ti; r+=1

    # ── EXPENSE SECTIONS ─────────────────────────────────────────────────────
    rt["subtotals"] = {}
    sub_rows_b = []; sub_rows_a = []

    for cat_name, sub_items in EXPENSE_CATS:
        merge_set(ws,r,1,r,NCOLS,cat_name,True,10,WHITE,SAGE_MID,"left",height=22); r+=1
        sub_start = r
        for j,sub in enumerate(sub_items):
            bg = SAGE_PALE if j%2 else CREAM
            set_cell(ws,r,1,cat_name,bg=bg,height=18)
            set_cell(ws,r,2,sub,bg=bg)
            inp(ws,r,3,0,FMT_CURR,bg); inp(ws,r,4,0,FMT_CURR,bg)
            frm(ws,r,5,f"=IFERROR(C{r}-D{r},0)",FMT_CURR,CHARCOAL,bg)
            frm(ws,r,6,f"=IFERROR(D{r}/C{r},0)",FMT_PCT,CHARCOAL,bg)
            set_cell(ws,r,7,"",bg=bg); r+=1
        sub_end = r-1

        sub_r = r
        merge_set(ws,sub_r,1,sub_r,2,f"{cat_name} TOTAL",True,9,CHARCOAL,SAGE_LIGHT,"left",
                  border_fn=bot_med,height=20)
        for col,f in [(3,f"=SUM(C{sub_start}:C{sub_end})"),(4,f"=SUM(D{sub_start}:D{sub_end})")]:
            fc=ws.cell(row=sub_r,column=col,value=f); fc.font=fn(9,True,CHARCOAL)
            fc.fill=fl(SAGE_LIGHT); fc.alignment=al("right"); fc.border=bot_med()
            fc.number_format=FMT_CURR
        frm(ws,sub_r,5,f"=IFERROR(C{sub_r}-D{sub_r},0)",FMT_CURR,CHARCOAL,SAGE_LIGHT)
        frm(ws,sub_r,6,f"=IFERROR(D{sub_r}/C{sub_r},0)",FMT_PCT,CHARCOAL,SAGE_LIGHT)
        set_cell(ws,sub_r,7,"",bg=SAGE_LIGHT)
        rt["subtotals"][cat_name] = sub_r
        sub_rows_b.append(f"C{sub_r}"); sub_rows_a.append(f"D{sub_r}")
        r+=1

    # Grand Total
    gt = r
    merge_set(ws,gt,1,gt,2,"GRAND TOTAL EXPENSES",True,10,WHITE,TERRA,"left",height=24)
    for col,f in [(3,f"=={'+'.join(sub_rows_b)}"),(4,f"=={'+'.join(sub_rows_a)}")]:
        # fix the double == from f-string
        formula = f[1:]  # remove leading =
        fc=ws.cell(row=gt,column=col,value=formula); fc.font=fn(10,True,WHITE)
        fc.fill=fl(TERRA); fc.alignment=al("right"); fc.border=tborder()
        fc.number_format=FMT_CURR
    frm(ws,gt,5,f"=IFERROR(C{gt}-D{gt},0)",FMT_CURR,WHITE,TERRA)
    for col in [6,7]:
        set_cell(ws,gt,col,"",bg=TERRA)
    rt["gt"] = gt; r+=1

    # Net Cash Flow
    ncf = r
    merge_set(ws,ncf,1,ncf,2,"NET CASH FLOW",True,10,WHITE,SAGE_DARK,"left",height=24)
    frm(ws,ncf,3,f"=IFERROR(C{ti}-C{gt},0)",FMT_CURR,WHITE,SAGE_DARK)
    frm(ws,ncf,4,f"=IFERROR(D{ti}-D{gt},0)",FMT_CURR,WHITE,SAGE_DARK)
    for col in [5,6,7]: set_cell(ws,ncf,col,"",bg=SAGE_DARK)
    rt["ncf"] = ncf; r+=1

    # ── BUDGET vs ACTUAL CHART (horizontal bar) ──────────────────────────────
    chart_row = r + 1
    cat_labels_col = 1
    budgeted_col   = 3
    actual_col     = 4

    # Write a small helper table for the chart (category names + subtotals)
    helper_start = r + 1
    merge_set(ws,r,1,r,4,"BUDGET VS ACTUAL — CATEGORY OVERVIEW",True,9,WHITE,SAGE_MID,"left",height=22)
    r+=1
    for i,(cat_name,_) in enumerate(EXPENSE_CATS):
        sub_r = rt["subtotals"][cat_name]
        set_cell(ws,r,1,cat_name,bold=True,bg=SAGE_PALE if i%2 else CREAM,height=18)
        frm(ws,r,2,f"=C{sub_r}",FMT_CURR,CHARCOAL,SAGE_PALE if i%2 else CREAM)
        frm(ws,r,3,f"=D{sub_r}",FMT_CURR,CHARCOAL,SAGE_PALE if i%2 else CREAM)
        r+=1
    helper_end = r-1

    bar = BarChart()
    bar.type = "bar"; bar.grouping = "clustered"; bar.style = 10
    bar.title = f"{month_name} · Budget vs Actual by Category"
    bar.y_axis.title = ""; bar.x_axis.title = "Amount"
    bar.height = 12; bar.width  = 20

    cats_ref  = Reference(ws, min_col=1, min_row=helper_start, max_row=helper_end)
    budg_ref  = Reference(ws, min_col=2, min_row=helper_start-1, max_row=helper_end)
    act_ref   = Reference(ws, min_col=3, min_row=helper_start-1, max_row=helper_end)
    bar.add_data(budg_ref, titles_from_data=True)
    bar.add_data(act_ref,  titles_from_data=True)
    bar.set_categories(cats_ref)
    try:
        bar.series[0].graphicalProperties.solidFill = SAGE_DARK
        bar.series[1].graphicalProperties.solidFill = TERRA
    except Exception: pass
    ws.add_chart(bar, f"E{helper_start}")

    freeze(ws,"A4")
    set_widths(ws,{"A":26,"B":24,"C":14,"D":14,"E":14,"F":10,"G":20})
    return rt


# ═══════════════════════════════════════════════════════════════════════════
# BUILD WORKBOOK
# ═══════════════════════════════════════════════════════════════════════════
wb = Workbook()
ws_dash = wb.active
ws_dash.title = "Dashboard"

# Build monthly sheets
month_meta = {}
for m_short, m_long in zip(MONTHS, MONTH_NAMES):
    ws_m = wb.create_sheet(title=m_short)
    month_meta[m_short] = build_month(ws_m, m_long, m_short)

# ═══════════════════════════════════════════════════════════════════════════
# DASHBOARD
# ═══════════════════════════════════════════════════════════════════════════
no_gridlines(ws_dash)
ws_dash.sheet_properties.tabColor = SAGE_DARK
DCOLS = 16

# Row 1-2: titles
merge_set(ws_dash,1,1,1,DCOLS,"PERSONAL FINANCE COMMAND CENTER · 2026",
          True,18,WHITE,SAGE_DARK,"center",height=48)
merge_set(ws_dash,2,1,2,DCOLS,"Live Summary Dashboard — All figures link to monthly sheets",
          False,9,WHITE,GOLD,"left",italic=True,height=20)

# ── KPI METRIC CARDS (rows 3-6, 4 cards × 4 cols each) ─────────────────────
jan_meta = month_meta["Jan"]

# Build YTD savings formula
sav_cat   = "SAVINGS & INVESTMENTS"
ytd_parts = [f"{m}!D{month_meta[m]['subtotals'][sav_cat]}" for m in MONTHS]
ytd_f     = "=IFERROR("+"+".join(ytd_parts)+",0)"

jan_gt  = month_meta["Jan"]["gt"]
jan_ti  = month_meta["Jan"]["ti"]

kpi_cards = [
    ("ANNUAL INCOME",   "=IFERROR(D5/12*12,0)",          FMT_CURR, SAGE_DARK),
    ("ANNUAL SAVINGS",  ytd_f,                            FMT_CURR, TERRA),
    ("LEFT TO SPEND",   f"=IFERROR(D5/12-Jan!D{jan_gt},0)", FMT_CURR, SAGE_DARK),
    ("SAVINGS RATE",    f"=IFERROR(({ytd_f[1:]})/IFERROR(D5,1),0)", FMT_PCT, TERRA),
]

for i,(label,formula,fmt,bg) in enumerate(kpi_cards):
    c1 = 1 + i*4; c2 = c1+3
    # Label row (row 3)
    merge_set(ws_dash,3,c1,3,c2,label,True,9,WHITE,bg,"center",height=20)
    # Value row (row 4) — card value
    mc = ws_dash.merge_cells(start_row=4,start_column=c1,end_row=5,end_column=c2)
    vc = ws_dash.cell(row=4,column=c1,value=formula)
    vc.font      = fn(16,True,WHITE)
    vc.fill      = fl(bg)
    vc.alignment = al("center","center")
    vc.border    = gold_bottom()
    vc.number_format = fmt
    ws_dash.row_dimensions[4].height = 28
    ws_dash.row_dimensions[5].height = 10

# Row 6 spacer
ws_dash.row_dimensions[6].height = 8

# ── BUDGET SUMMARY TABLE (rows 7-14, cols 1-5) ──────────────────────────────
merge_set(ws_dash,7,1,7,5,"BUDGET SUMMARY",True,10,WHITE,SAGE_DARK,"left",height=22)
for i,h in enumerate(["METRIC","","VALUE","",""]):
    set_cell(ws_dash,8,i+1,h,True,9,WHITE,SAGE_MID,"center",height=20)

# Annual income input (D5 reserved → put annual income at row 9 col 4)
budget_rows = [
    ("Annual Income",      True,  "D",  None,                               FMT_CURR, INPUT_BLUE),
    ("Monthly Income",     False, "D",  "=IFERROR(D9/12,0)",                FMT_CURR, CHARCOAL),
    ("This Month Expenses",False, "D",  f"=IFERROR(Jan!D{jan_gt},0)",       FMT_CURR, GREEN_OK),
    ("Left to Spend",      False, "D",  "=IFERROR(D10-D11,0)",              FMT_CURR, CHARCOAL),
    ("Annual Savings Rate",False, "D",  "=IFERROR(D12/D9,0)",               FMT_PCT,  CHARCOAL),
    ("YTD Total Savings",  False, "D",  ytd_f,                              FMT_CURR, GREEN_OK),
]
ANNUAL_INCOME_ROW = 9
for i,(lbl_t,is_inp,_,formula,fmt,color) in enumerate(budget_rows):
    r = 9+i
    bg = SAGE_PALE if i%2 else CREAM
    ws_dash.merge_cells(start_row=r,start_column=1,end_row=r,end_column=3)
    set_cell(ws_dash,r,1,lbl_t,True,9,CHARCOAL,bg,"left",height=18)
    for cx in [2,3]:
        ws_dash.cell(row=r,column=cx).fill=fl(bg)
        ws_dash.cell(row=r,column=cx).border=tborder()
    ws_dash.merge_cells(start_row=r,start_column=4,end_row=r,end_column=5)
    if is_inp:
        inp(ws_dash,r,4,0,fmt,bg)
    else:
        frm(ws_dash,r,4,formula,fmt,color,bg)
    ws_dash.cell(row=r,column=5).fill=fl(bg); ws_dash.cell(row=r,column=5).border=tborder()

# ── SAVINGS SNAPSHOT (rows 7-14, cols 7-11) ──────────────────────────────────
merge_set(ws_dash,7,7,7,11,"SAVINGS SNAPSHOT",True,10,WHITE,TERRA,"left",height=22)
for i,h in enumerate(["GOAL","","AMOUNT","",""]):
    set_cell(ws_dash,8,7+i,h,True,9,WHITE,TERRA,"center",height=20)

snap = [
    ("Emergency Fund Goal",   True,  None,                      FMT_CURR),
    ("Emergency Fund Current",True,  None,                      FMT_CURR),
    ("% Funded",              False, "=IFERROR(H11/H10,0)",     FMT_PCT),
    ("Monthly Target",        True,  None,                      FMT_CURR),
    ("YTD Saved",             False, ytd_f,                     FMT_CURR),
]
for i,(sl,si,sf,sfmt) in enumerate(snap):
    r = 9+i
    bg = TERRA_LIGHT if i%2 else CREAM
    ws_dash.merge_cells(start_row=r,start_column=7,end_row=r,end_column=9)
    set_cell(ws_dash,r,7,sl,True,9,CHARCOAL,bg,"left",height=18)
    for cx in [8,9]:
        ws_dash.cell(row=r,column=cx).fill=fl(bg); ws_dash.cell(row=r,column=cx).border=tborder()
    ws_dash.merge_cells(start_row=r,start_column=10,end_row=r,end_column=11)
    if si: inp(ws_dash,r,10,0,sfmt,bg)
    else:  frm(ws_dash,r,10,sf,sfmt,GREEN_OK,bg)
    ws_dash.cell(row=r,column=11).fill=fl(bg); ws_dash.cell(row=r,column=11).border=tborder()

# Row 15 spacer
ws_dash.row_dimensions[15].height = 8

# ── TOP CATEGORIES TABLE (rows 16-22, cols 1-5) ──────────────────────────────
merge_set(ws_dash,16,1,16,5,"TOP EXPENSE CATEGORIES — JANUARY",True,10,WHITE,SAGE_MID,"left",height=22)
for i,h in enumerate(["CATEGORY","BUDGETED","ACTUAL","DIFF",""]):
    set_cell(ws_dash,17,i+1,h,True,9,WHITE,SAGE_DARK,"center",height=20)

for i,(cat,_) in enumerate(EXPENSE_CATS[:6]):
    r = 18+i
    sr = month_meta["Jan"]["subtotals"][cat]
    bg = SAGE_PALE if i%2 else CREAM
    set_cell(ws_dash,r,1,cat,bold=True,bg=bg,height=18)
    frm(ws_dash,r,2,f"=IFERROR(Jan!C{sr},0)",FMT_CURR,GREEN_OK,bg)
    frm(ws_dash,r,3,f"=IFERROR(Jan!D{sr},0)",FMT_CURR,GREEN_OK,bg)
    frm(ws_dash,r,4,f"=IFERROR(B{r}-C{r},0)",FMT_CURR,CHARCOAL,bg)
    set_cell(ws_dash,r,5,"",bg=bg)

# ── BILLS TABLE (rows 16-22, cols 7-11) ──────────────────────────────────────
merge_set(ws_dash,16,7,16,11,"BILLS THIS MONTH",True,10,WHITE,TERRA,"left",height=22)
for i,h in enumerate(["BILL NAME","AMOUNT","DUE DATE","",""]):
    set_cell(ws_dash,17,7+i,h,True,9,WHITE,TERRA,"center",height=20)

sample_bills = [
    ("Rent/Mortgage",1200,"01/01/2026"),
    ("Electricity",80,"01/15/2026"),
    ("Internet",60,"01/22/2026"),
    ("Streaming",18,"01/08/2026"),
    ("Phone",70,"01/12/2026"),
    ("Car Insurance",120,"01/20/2026"),
]
for i,(bn,ba,bd) in enumerate(sample_bills):
    r = 18+i
    bg = TERRA_LIGHT if i%2 else CREAM
    set_cell(ws_dash,r,7,bn,bg=bg,height=18)
    inp(ws_dash,r,8,ba,FMT_CURR,bg)
    cd = ws_dash.cell(row=r,column=9,value=bd)
    cd.font=fn(9,False,INPUT_BLUE); cd.fill=fl(bg)
    cd.alignment=al("right"); cd.border=tborder(); cd.number_format=FMT_DATE
    set_cell(ws_dash,r,10,"",bg=bg); set_cell(ws_dash,r,11,"",bg=bg)

# Row 24 spacer
ws_dash.row_dimensions[24].height = 8

# ── MINI BILL CALENDAR (rows 25-34, cols 1-8) ────────────────────────────────
CAL_YEAR = 2026; CAL_MONTH = 1
bill_due_days = {1,8,12,15,20,22}   # from sample_bills day numbers

merge_set(ws_dash,25,1,25,8,"JANUARY 2026 — BILL CALENDAR",True,11,WHITE,SAGE_DARK,"center",height=24)
day_names = ["MON","TUE","WED","THU","FRI","SAT","SUN"]
for i,d in enumerate(day_names):
    set_cell(ws_dash,26,i+1,d,True,9,WHITE,SAGE_MID,"center",height=20)

month_grid = calendar.monthcalendar(CAL_YEAR, CAL_MONTH)
for week_i, week in enumerate(month_grid):
    r = 27 + week_i
    ws_dash.row_dimensions[r].height = 18
    for day_i, day in enumerate(week):
        col = day_i + 1
        if day == 0:
            set_cell(ws_dash,r,col,"",bg=CREAM)
        elif day in bill_due_days:
            set_cell(ws_dash,r,col,day,False,9,TERRA,TERRA_LIGHT,"center")
        else:
            set_cell(ws_dash,r,col,day,False,9,CHARCOAL,SAGE_PALE,"center")

# ── KEY STATS BOX (rows 25-34, cols 10-14) ───────────────────────────────────
merge_set(ws_dash,25,10,25,14,"KEY STATS",True,11,WHITE,TERRA,"center",height=24)
# Use simple cross-sheet formulas for key stats
key_stats = [
    ("Monthly Income",    f"=IFERROR(D10,0)",    FMT_CURR),
    ("Monthly Expenses",  f"=IFERROR(D11,0)",    FMT_CURR),
    ("Monthly Savings",   f"=IFERROR(D12,0)",    FMT_CURR),
    ("Savings Rate",      f"=IFERROR(D13,0)",    FMT_PCT),
    ("YTD Savings",       ytd_f,                 FMT_CURR),
    ("Emergency %",       "=IFERROR(H11/H10,0)", FMT_PCT),
]
for i,(sl,sf,sfmt) in enumerate(key_stats):
    r = 26+i
    bg = SAGE_PALE if i%2 else CREAM
    ws_dash.merge_cells(start_row=r,start_column=10,end_row=r,end_column=12)
    set_cell(ws_dash,r,10,sl,True,9,CHARCOAL,bg,"left",height=18)
    for cx in [11,12]:
        ws_dash.cell(row=r,column=cx).fill=fl(bg)
        ws_dash.cell(row=r,column=cx).border=tborder()
    ws_dash.merge_cells(start_row=r,start_column=13,end_row=r,end_column=14)
    frm(ws_dash,r,13,sf,sfmt,GREEN_OK,bg)
    ws_dash.cell(row=r,column=14).fill=fl(bg)
    ws_dash.cell(row=r,column=14).border=tborder()

# Row 33-34 spacers
for r in [33,34]: ws_dash.row_dimensions[r].height = 8

# ── DONUT CHART — Expense Distribution ───────────────────────────────────────
# Helper table at rows 38-46 cols 18-19 (below calendar, far right)
D_ROW = 38; D_COL = 18
for i,(cat,_) in enumerate(EXPENSE_CATS):
    sr = month_meta["Jan"]["subtotals"][cat]
    ws_dash.cell(row=D_ROW+i, column=D_COL).value   = cat
    fc = ws_dash.cell(row=D_ROW+i, column=D_COL+1,
                      value=f"=IFERROR(Jan!D{sr},0)")
    fc.number_format = FMT_CURR

donut = DoughnutChart()
donut.title   = "Expense Distribution — January"
donut.style   = 10
donut.holeSize= 50
donut.height  = 12; donut.width = 14

d_labels = Reference(ws_dash, min_col=D_COL,   min_row=D_ROW, max_row=D_ROW+len(EXPENSE_CATS)-1)
d_values = Reference(ws_dash, min_col=D_COL+1, min_row=D_ROW, max_row=D_ROW+len(EXPENSE_CATS)-1)
donut.add_data(d_values)
donut.set_categories(d_labels)
ws_dash.add_chart(donut,"F7")

# ── SAVINGS OVER TIME BAR CHART ───────────────────────────────────────────────
# Helper table at cols 18-19 (far right, outside visible layout)
H1C = 18  # helper col base
ws_dash.cell(row=50,column=H1C).value="Month"
ws_dash.cell(row=50,column=H1C+1).value="Savings"
for i,m in enumerate(MONTHS):
    sr = month_meta[m]["subtotals"][sav_cat]
    ws_dash.cell(row=51+i,column=H1C).value = m
    fc = ws_dash.cell(row=51+i,column=H1C+1,value=f"=IFERROR({m}!D{sr},0)")
    fc.number_format=FMT_CURR

sav_bar = BarChart()
sav_bar.type="col"; sav_bar.style=10; sav_bar.grouping="clustered"
sav_bar.title="Monthly Savings"; sav_bar.height=10; sav_bar.width=14
sav_ref  = Reference(ws_dash,min_col=H1C+1,min_row=50,max_row=62)
sav_cats = Reference(ws_dash,min_col=H1C,  min_row=51,max_row=62)
sav_bar.add_data(sav_ref,titles_from_data=True)
sav_bar.set_categories(sav_cats)
try: sav_bar.series[0].graphicalProperties.solidFill = SAGE_DARK
except Exception: pass
ws_dash.add_chart(sav_bar,"K16")

# ── ANNUAL INCOME vs EXPENSES BAR CHART ──────────────────────────────────────
# Helper table at cols 21-23 rows 50-63
H2C = 21
ws_dash.cell(row=50,column=H2C).value="Month"
ws_dash.cell(row=50,column=H2C+1).value="Income"
ws_dash.cell(row=50,column=H2C+2).value="Expenses"
for i,m in enumerate(MONTHS):
    ti_r=month_meta[m]["ti"]; gt_r=month_meta[m]["gt"]
    ws_dash.cell(row=51+i,column=H2C).value=m
    fi=ws_dash.cell(row=51+i,column=H2C+1,value=f"=IFERROR({m}!D{ti_r},0)")
    fi.number_format=FMT_CURR
    fe=ws_dash.cell(row=51+i,column=H2C+2,value=f"=IFERROR({m}!D{gt_r},0)")
    fe.number_format=FMT_CURR

ann_bar = BarChart()
ann_bar.type="col"; ann_bar.style=10; ann_bar.grouping="clustered"
ann_bar.title="Annual Income vs Expenses"; ann_bar.height=14; ann_bar.width=24
ann_inc = Reference(ws_dash,min_col=H2C+1,min_row=50,max_row=62)
ann_exp = Reference(ws_dash,min_col=H2C+2,min_row=50,max_row=62)
ann_cat = Reference(ws_dash,min_col=H2C,  min_row=51,max_row=62)
ann_bar.add_data(ann_inc,titles_from_data=True)
ann_bar.add_data(ann_exp,titles_from_data=True)
ann_bar.set_categories(ann_cat)
try:
    ann_bar.series[0].graphicalProperties.solidFill = SAGE_DARK
    ann_bar.series[1].graphicalProperties.solidFill = TERRA
except Exception: pass
ws_dash.add_chart(ann_bar,"A35")

freeze(ws_dash,"A3")
set_widths(ws_dash,{
    "A":24,"B":13,"C":13,"D":13,"E":3,"F":24,"G":13,"H":13,"I":13,"J":3,
    "K":20,"L":13,"M":13,"N":13,"O":13,"P":3
})


# ═══════════════════════════════════════════════════════════════════════════
# DEBT TRACKER
# ═══════════════════════════════════════════════════════════════════════════
ws_debt = wb.create_sheet("Debt Tracker")
no_gridlines(ws_debt); ws_debt.sheet_properties.tabColor = TERRA
DCOLS_DEBT = 9
merge_set(ws_debt,1,1,1,DCOLS_DEBT,"DEBT PAYOFF TRACKER",True,18,WHITE,SAGE_DARK,"center",height=48)
merge_set(ws_debt,2,1,2,DCOLS_DEBT,"Track balances, interest rates, and payoff timelines",
          False,9,WHITE,GOLD,"left",italic=True,height=20)
for i,h in enumerate(["DEBT NAME","LENDER","ORIGINAL BAL","CURRENT BAL",
                       "INTEREST %","MIN PAYMENT","MONTHLY PMT","EST PAYOFF","STATUS"]):
    set_cell(ws_debt,3,i+1,h,True,9,WHITE,SAGE_DARK,"center",height=20)

debts = ["Credit Card 1","Credit Card 2","Student Loan","Car Loan","Personal Loan","Other Debt"]
for j,debt in enumerate(debts):
    r=4+j; bg=TERRA_LIGHT if j%2 else CREAM
    set_cell(ws_debt,r,1,debt,bg=bg,height=18)
    inp(ws_debt,r,2,"","@",bg); ws_debt.cell(row=r,column=2).number_format="@"
    inp(ws_debt,r,3,0,FMT_CURR,bg); inp(ws_debt,r,4,0,FMT_CURR,bg)
    inp(ws_debt,r,5,0,FMT_PCT,bg); inp(ws_debt,r,6,0,FMT_CURR,bg)
    inp(ws_debt,r,7,0,FMT_CURR,bg); inp(ws_debt,r,8,"",FMT_DATE,bg)
    set_cell(ws_debt,r,9,"Active",bg=bg)

dt_end = 4+len(debts)-1; dt_tot = dt_end+1
merge_set(ws_debt,dt_tot,1,dt_tot,2,"TOTALS",True,10,WHITE,TERRA,"left",height=24)
for col,f in [(3,f"=SUM(C4:C{dt_end})"),(4,f"=SUM(D4:D{dt_end})"),
              (6,f"=SUM(F4:F{dt_end})"),(7,f"=SUM(G4:G{dt_end})")]:
    fc=ws_debt.cell(row=dt_tot,column=col,value=f)
    fc.font=fn(10,True,WHITE); fc.fill=fl(TERRA); fc.alignment=al("right")
    fc.border=tborder(); fc.number_format=FMT_CURR
for col in [5,8,9]: set_cell(ws_debt,dt_tot,col,"",bg=TERRA)

sb=dt_tot+2
merge_set(ws_debt,sb,1,sb,5,"DEBT SUMMARY",True,10,WHITE,SAGE_DARK,"left",height=22)
for k,(sl,sf,sfmt) in enumerate([
    ("Total Outstanding Debt",   f"=IFERROR(SUM(D4:D{dt_end}),0)",    FMT_CURR),
    ("Total Monthly Minimums",   f"=IFERROR(SUM(F4:F{dt_end}),0)",    FMT_CURR),
    ("Debt-to-Income Ratio",     f"=IFERROR(SUM(D4:D{dt_end})/Dashboard!D9*12,0)", FMT_PCT),
    ("Highest Interest Debt",    f'=IFERROR(INDEX(A4:A{dt_end},MATCH(MAX(E4:E{dt_end}),E4:E{dt_end},0)),"N/A")', "@"),
]):
    r=sb+1+k; bg=SAGE_PALE if k%2 else CREAM
    ws_debt.merge_cells(start_row=r,start_column=1,end_row=r,end_column=3)
    set_cell(ws_debt,r,1,sl,True,9,CHARCOAL,bg,"left",height=18)
    for cx in [2,3]: ws_debt.cell(row=r,column=cx).fill=fl(bg); ws_debt.cell(row=r,column=cx).border=tborder()
    ws_debt.merge_cells(start_row=r,start_column=4,end_row=r,end_column=5)
    frm(ws_debt,r,4,sf,sfmt,GREEN_OK,bg)
    ws_debt.cell(row=r,column=5).fill=fl(bg); ws_debt.cell(row=r,column=5).border=tborder()

freeze(ws_debt,"A3")
set_widths(ws_debt,{"A":22,"B":18,"C":14,"D":14,"E":12,"F":14,"G":14,"H":14,"I":12})


# ═══════════════════════════════════════════════════════════════════════════
# BILL CALENDAR
# ═══════════════════════════════════════════════════════════════════════════
ws_bill = wb.create_sheet("Bill Calendar")
no_gridlines(ws_bill); ws_bill.sheet_properties.tabColor = TERRA
BCOLS = 9
merge_set(ws_bill,1,1,1,BCOLS,"BILL CALENDAR · 2026",True,18,WHITE,SAGE_DARK,"center",height=48)
merge_set(ws_bill,2,1,2,BCOLS,"Recurring bills, auto-payments, and annual cost summary",
          False,9,WHITE,GOLD,"left",italic=True,height=20)
for i,h in enumerate(["BILL NAME","CATEGORY","AMOUNT","DUE DATE",
                       "FREQUENCY","AUTO-PAY","PAID?","ANNUAL COST","NOTES"]):
    set_cell(ws_bill,3,i+1,h,True,9,WHITE,SAGE_DARK,"center",height=20)

bills_data = [
    ("Rent/Mortgage","Housing",1200,"01/01/2026","Monthly","No","No"),
    ("Electricity","Utilities",80,"01/15/2026","Monthly","Yes","No"),
    ("Internet","Utilities",60,"01/22/2026","Monthly","Yes","No"),
    ("Phone","Subscriptions",70,"01/12/2026","Monthly","Yes","No"),
    ("Car Insurance","Insurance",120,"01/20/2026","Monthly","No","No"),
    ("Streaming","Subscriptions",18,"01/08/2026","Monthly","Yes","No"),
    ("Gym Membership","Health",45,"01/01/2026","Monthly","Yes","No"),
    ("Spotify","Subscriptions",11,"01/05/2026","Monthly","Yes","No"),
    ("Home Insurance","Insurance",150,"01/01/2026","Annual","No","No"),
    ("Car Registration","Transport",85,"03/01/2026","Annual","No","No"),
]
for j,(bn,bc,ba,bd,bf,bap,bpd) in enumerate(bills_data):
    r=4+j; bg=TERRA_LIGHT if j%2 else CREAM
    set_cell(ws_bill,r,1,bn,bg=bg,height=18); set_cell(ws_bill,r,2,bc,bg=bg)
    inp(ws_bill,r,3,ba,FMT_CURR,bg)
    cd=ws_bill.cell(row=r,column=4,value=bd); cd.font=fn(9,False,INPUT_BLUE)
    cd.fill=fl(bg); cd.alignment=al("right"); cd.border=tborder(); cd.number_format=FMT_DATE
    set_cell(ws_bill,r,5,bf,bg=bg); set_cell(ws_bill,r,6,bap,bg=bg); set_cell(ws_bill,r,7,bpd,bg=bg)
    ac=ws_bill.cell(row=r,column=8,
        value=f'=IFERROR(IF(E{r}="Monthly",C{r}*12,IF(E{r}="Annual",C{r},IF(E{r}="Quarterly",C{r}*4,C{r}*52))),0)')
    ac.font=fn(9,False,CHARCOAL); ac.fill=fl(bg); ac.alignment=al("right")
    ac.border=tborder(); ac.number_format=FMT_CURR
    set_cell(ws_bill,r,9,"",bg=bg)

be=4+len(bills_data)-1; bt=be+1
merge_set(ws_bill,bt,1,bt,2,"MONTHLY TOTAL",True,10,WHITE,TERRA,"left",height=24)
for col,f in [(3,f"=SUM(C4:C{be})"),(8,f"=SUM(H4:H{be})")]:
    fc=ws_bill.cell(row=bt,column=col,value=f); fc.font=fn(10,True,WHITE)
    fc.fill=fl(TERRA); fc.alignment=al("right"); fc.border=tborder(); fc.number_format=FMT_CURR
for col in [4,5,6,7,9]: set_cell(ws_bill,bt,col,"",bg=TERRA)

for dv_range, formula1 in [
    (f"B4:B{be}", '"Housing,Utilities,Insurance,Subscriptions,Health,Transport,Other"'),
    (f"E4:E{be}", '"Monthly,Weekly,Bi-Weekly,Quarterly,Annual"'),
    (f"F4:G{be}", '"Yes,No"'),
]:
    dv=DataValidation(type="list",formula1=formula1,allow_blank=True)
    ws_bill.add_data_validation(dv); dv.sqref=dv_range

freeze(ws_bill,"A3")
set_widths(ws_bill,{"A":22,"B":16,"C":12,"D":12,"E":12,"F":10,"G":8,"H":14,"I":22})


# ═══════════════════════════════════════════════════════════════════════════
# SAVINGS GOALS
# ═══════════════════════════════════════════════════════════════════════════
ws_sav = wb.create_sheet("Savings Goals")
no_gridlines(ws_sav); ws_sav.sheet_properties.tabColor = SAGE_DARK
SCOLS = 8
merge_set(ws_sav,1,1,1,SCOLS,"SAVINGS GOALS TRACKER",True,18,WHITE,SAGE_DARK,"center",height=48)
merge_set(ws_sav,2,1,2,SCOLS,"Monitor goals, progress bars, and projected completion dates",
          False,9,WHITE,GOLD,"left",italic=True,height=20)
for i,h in enumerate(["GOAL","TARGET","SAVED","MONTHLY","TARGET DATE","% DONE","PROGRESS","PRIORITY"]):
    set_cell(ws_sav,3,i+1,h,True,9,WHITE,SAGE_DARK,"center",height=20)

goals=[
    ("Emergency Fund",10000,0,500,"12/31/2026","High"),
    ("Vacation",3000,0,200,"07/01/2026","Medium"),
    ("Home Down Payment",50000,0,800,"01/01/2029","High"),
    ("New Car",15000,0,300,"06/01/2027","Medium"),
    ("Education",8000,0,250,"09/01/2027","Medium"),
    ("Retirement Boost",20000,0,400,"01/01/2030","Low"),
]
pri_bg={"High":TERRA_LIGHT,"Medium":SAGE_PALE,"Low":CREAM}
for j,(gn,gt_,gc,gm,gd,gp) in enumerate(goals):
    r=4+j; bg=pri_bg.get(gp,CREAM)
    set_cell(ws_sav,r,1,gn,True,9,CHARCOAL,bg,"left",height=18)
    inp(ws_sav,r,2,gt_,FMT_CURR,bg); inp(ws_sav,r,3,gc,FMT_CURR,bg); inp(ws_sav,r,4,gm,FMT_CURR,bg)
    cd=ws_sav.cell(row=r,column=5,value=gd); cd.font=fn(9,False,INPUT_BLUE)
    cd.fill=fl(bg); cd.alignment=al("right"); cd.border=tborder(); cd.number_format=FMT_DATE
    frm(ws_sav,r,6,f"=IFERROR(C{r}/B{r},0)",FMT_PCT,CHARCOAL,bg)
    pb=ws_sav.cell(row=r,column=7,
        value=f'=IFERROR(REPT(CHAR(9608),ROUND(F{r}*10,0))&REPT(CHAR(9617),10-ROUND(F{r}*10,0)),"          ")')
    pb.font=fn(9,False,SAGE_DARK); pb.fill=fl(bg); pb.alignment=al("left"); pb.border=tborder()
    set_cell(ws_sav,r,8,gp,True,9,CHARCOAL,bg,"center")

goal_end=4+len(goals)-1; tot_sg=goal_end+1
for col_,val_,fmt_ in [
    (1,"TOTALS","@"),(2,f"=SUM(B4:B{goal_end})",FMT_CURR),
    (3,f"=SUM(C4:C{goal_end})",FMT_CURR),(4,f"=SUM(D4:D{goal_end})",FMT_CURR),
    (6,f"=IFERROR(SUM(C4:C{goal_end})/SUM(B4:B{goal_end}),0)",FMT_PCT),
]:
    c=ws_sav.cell(row=tot_sg,column=col_,value=val_)
    c.font=fn(9,True,CHARCOAL); c.fill=fl(SAGE_LIGHT); c.alignment=al("right" if col_>1 else "left")
    c.border=tborder(); c.number_format=fmt_
for col_ in [5,7,8]:
    c=ws_sav.cell(row=tot_sg,column=col_,value="")
    c.fill=fl(SAGE_LIGHT); c.border=tborder()
ws_sav.row_dimensions[tot_sg].height=20

# Progress horizontal bar chart
sav_prog = BarChart()
sav_prog.type="bar"; sav_prog.style=10; sav_prog.grouping="clustered"
sav_prog.title="Savings Goal Progress"; sav_prog.height=10; sav_prog.width=16
pg_labels = Reference(ws_sav,min_col=1,min_row=4,max_row=goal_end)
pg_values = Reference(ws_sav,min_col=6,min_row=3,max_row=goal_end)
sav_prog.add_data(pg_values,titles_from_data=True)
sav_prog.set_categories(pg_labels)
try: sav_prog.series[0].graphicalProperties.solidFill = SAGE_DARK
except Exception: pass
ws_sav.add_chart(sav_prog,"J4")

dv_p=DataValidation(type="list",formula1='"High,Medium,Low"',allow_blank=True)
ws_sav.add_data_validation(dv_p); dv_p.sqref=f"H4:H{goal_end}"

freeze(ws_sav,"A3")
set_widths(ws_sav,{"A":22,"B":14,"C":14,"D":14,"E":12,"F":10,"G":16,"H":10})


# ═══════════════════════════════════════════════════════════════════════════
# SPENDING TRACKER  — "Days Without Spending"
# ═══════════════════════════════════════════════════════════════════════════
ws_sp = wb.create_sheet("Spending Tracker")
no_gridlines(ws_sp); ws_sp.sheet_properties.tabColor = SAGE_DARK
merge_set(ws_sp,1,1,1,33,"DAYS WITHOUT SPENDING — 2026",True,18,WHITE,SAGE_DARK,"center",height=48)
merge_set(ws_sp,2,1,2,33,"Mark X on days you spent nothing — track your no-spend streaks",
          False,9,WHITE,GOLD,"left",italic=True,height=20)

# Column headers: col A = month label, cols B-AF = days 1-31, col AG = count
set_cell(ws_sp,3,1,"MONTH",True,9,WHITE,SAGE_DARK,"center",height=20)
for d in range(1,32):
    set_cell(ws_sp,3,d+1,d,True,9,WHITE,SAGE_MID,"center",height=20)
set_cell(ws_sp,3,33,"NO-SPEND DAYS",True,9,WHITE,SAGE_DARK,"center",height=20)

# Days in each month for 2026
days_in_month = [31,28,31,30,31,30,31,31,30,31,30,31]  # 2026 non-leap

for mi,(m_short,m_long,days) in enumerate(zip(MONTHS,MONTH_NAMES,days_in_month)):
    r = 4 + mi*2          # 2 rows per month: day headers (light) + input row
    r_input = r+1

    # Month label spanning 2 rows
    ws_sp.merge_cells(start_row=r,start_column=1,end_row=r_input,end_column=1)
    mc=ws_sp.cell(row=r,column=1,value=m_long)
    mc.font=fn(9,True,WHITE); mc.fill=fl(SAGE_DARK if mi%2==0 else TERRA)
    mc.alignment=al("center","center"); mc.border=tborder()
    ws_sp.row_dimensions[r].height=14
    ws_sp.row_dimensions[r_input].height=18

    for d in range(1,32):
        col=d+1
        if d <= days:
            # Day number header (tiny, top row)
            dc=ws_sp.cell(row=r,column=col,value=d)
            dc.font=fn(8,False,CHARCOAL); dc.fill=fl(SAGE_PALE if mi%2==0 else TERRA_LIGHT)
            dc.alignment=al("center","center"); dc.border=tborder()
            # Input cell (bottom row)
            ic=ws_sp.cell(row=r_input,column=col,value="")
            ic.font=fn(9,False,INPUT_BLUE); ic.fill=fl(WHITE)
            ic.alignment=al("center","center"); ic.border=tborder()
        else:
            # Past month's days — fill cream/disabled
            for row_ in [r,r_input]:
                cc=ws_sp.cell(row=row_,column=col,value="")
                cc.fill=fl(CREAM); cc.border=tborder()

    # Count formula =COUNTIF(B{r_input}:AF{r_input},"X")
    cnt_col=33
    cf=ws_sp.cell(row=r_input,column=cnt_col,
        value=f'=COUNTIF(B{r_input}:{get_column_letter(32)}{r_input},"X")')
    cf.font=fn(9,True,GREEN_OK); cf.fill=fl(SAGE_LIGHT); cf.alignment=al("center")
    cf.border=tborder(); cf.number_format='0 "days"'
    # Merge top row count col
    cc=ws_sp.cell(row=r,column=cnt_col,value="")
    cc.fill=fl(SAGE_LIGHT); cc.border=tborder()

# Annual total no-spend days
ann_r = 4 + len(MONTHS)*2
merge_set(ws_sp,ann_r,1,ann_r,32,"ANNUAL TOTAL NO-SPEND DAYS",True,10,WHITE,SAGE_DARK,"left",height=24)
count_refs = "+".join([f"{get_column_letter(33)}{4+mi*2+1}" for mi in range(12)])
cf_ann=ws_sp.cell(row=ann_r,column=33,value=f"={count_refs}")
cf_ann.font=fn(10,True,WHITE); cf_ann.fill=fl(SAGE_DARK)
cf_ann.alignment=al("center"); cf_ann.border=tborder()

freeze(ws_sp,"B3")
# Column widths: A wide, B-AF narrow, AG wider
set_widths(ws_sp,{"A":14,**{get_column_letter(d+1):4 for d in range(1,32)},"AG":14})


# ═══════════════════════════════════════════════════════════════════════════
# ANNUAL OVERVIEW
# ═══════════════════════════════════════════════════════════════════════════
ws_ann = wb.create_sheet("Annual Overview")
no_gridlines(ws_ann); ws_ann.sheet_properties.tabColor = SAGE_DARK
ANN_COLS = 16
merge_set(ws_ann,1,1,1,ANN_COLS,"ANNUAL OVERVIEW · 2026",True,18,WHITE,SAGE_DARK,"center",height=48)
merge_set(ws_ann,2,1,2,ANN_COLS,"Year-at-a-glance consolidated from all 12 monthly sheets",
          False,9,WHITE,GOLD,"left",italic=True,height=20)

ann_hdrs = ["LINE ITEM"]+MONTHS+["ANNUAL TOTAL","ANNUAL BUDGET","VARIANCE"]
for i,h in enumerate(ann_hdrs):
    set_cell(ws_ann,3,i+1,h,True,9,WHITE,SAGE_DARK if i in [0,13,14,15] else SAGE_MID,"center",height=20)

ann_rows = [
    ("INCOME",                  "ti",  None),
    ("HOUSING",                 "sub", "HOUSING"),
    ("TRANSPORT",               "sub", "TRANSPORT"),
    ("FOOD & DINING",           "sub", "FOOD & DINING"),
    ("HEALTH & WELLNESS",       "sub", "HEALTH & WELLNESS"),
    ("LIFESTYLE",               "sub", "LIFESTYLE"),
    ("PERSONAL",                "sub", "PERSONAL"),
    ("SAVINGS & INVESTMENTS",   "sub", "SAVINGS & INVESTMENTS"),
    ("DEBT PAYMENTS",           "sub", "DEBT PAYMENTS"),
    ("GRAND TOTAL EXPENSES",    "gt",  None),
    ("NET CASH FLOW",           "ncf", None),
]

income_ann_row = None; expenses_ann_row = None; ncf_ann_row = None

for i,(label,key,cat) in enumerate(ann_rows):
    r = 4+i
    is_key = label in ("INCOME","GRAND TOTAL EXPENSES","NET CASH FLOW")
    bg_map = {"INCOME":SAGE_LIGHT,"GRAND TOTAL EXPENSES":TERRA,"NET CASH FLOW":SAGE_DARK}
    bg  = bg_map.get(label, SAGE_PALE if i%2 else CREAM)
    txt = WHITE if label in ("GRAND TOTAL EXPENSES","NET CASH FLOW") else CHARCOAL

    set_cell(ws_ann,r,1,label,is_key,9,txt,bg,"left",height=18 if not is_key else 22)

    for mi,m in enumerate(MONTHS):
        col = 2+mi
        meta = month_meta[m]
        sr = meta["ti"] if key=="ti" else meta["gt"] if key=="gt" else \
             meta["ncf"] if key=="ncf" else meta["subtotals"][cat]
        fc=ws_ann.cell(row=r,column=col,value=f"=IFERROR({m}!D{sr},0)")
        fc.font=fn(9,is_key,GREEN_OK); fc.fill=fl(bg); fc.alignment=al("right")
        fc.border=tborder(); fc.number_format=FMT_CURR

    # Annual total
    fc_n=ws_ann.cell(row=r,column=14,value=f"=SUM(B{r}:M{r})")
    fc_n.font=fn(9,True,txt); fc_n.fill=fl(bg); fc_n.alignment=al("right")
    fc_n.border=tborder(); fc_n.number_format=FMT_CURR
    inp(ws_ann,r,15,0,FMT_CURR,bg)
    fc_v=ws_ann.cell(row=r,column=16,value=f"=IFERROR(N{r}-O{r},0)")
    fc_v.font=fn(9,True,txt); fc_v.fill=fl(bg); fc_v.alignment=al("right")
    fc_v.border=tborder(); fc_v.number_format=FMT_CURR

    if label=="INCOME":             income_ann_row=r
    if label=="GRAND TOTAL EXPENSES": expenses_ann_row=r
    if label=="NET CASH FLOW":      ncf_ann_row=r

# KEY METRICS
km=4+len(ann_rows)+2
merge_set(ws_ann,km,1,km,5,"KEY METRICS",True,10,WHITE,SAGE_DARK,"left",height=22)
km_items=[
    ("Best Month (lowest spend)",
     f'=IFERROR(INDEX(B3:M3,MATCH(MIN(B{expenses_ann_row}:M{expenses_ann_row}),B{expenses_ann_row}:M{expenses_ann_row},0)),"N/A")',"@"),
    ("Worst Month (highest spend)",
     f'=IFERROR(INDEX(B3:M3,MATCH(MAX(B{expenses_ann_row}:M{expenses_ann_row}),B{expenses_ann_row}:M{expenses_ann_row},0)),"N/A")',"@"),
    ("Annual Savings Rate",
     f"=IFERROR((N{income_ann_row}-N{expenses_ann_row})/N{income_ann_row},0)",FMT_PCT),
    ("Average Monthly Spend",
     f"=IFERROR(N{expenses_ann_row}/12,0)",FMT_CURR),
]
for k_,(sl,sf,sfmt) in enumerate(km_items):
    r=km+1+k_; bg=SAGE_PALE if k_%2 else CREAM
    ws_ann.merge_cells(start_row=r,start_column=1,end_row=r,end_column=3)
    set_cell(ws_ann,r,1,sl,True,9,CHARCOAL,bg,"left",height=18)
    for cx in [2,3]: ws_ann.cell(row=r,column=cx).fill=fl(bg); ws_ann.cell(row=r,column=cx).border=tborder()
    ws_ann.merge_cells(start_row=r,start_column=4,end_row=r,end_column=5)
    frm(ws_ann,r,4,sf,sfmt,GREEN_OK,bg)
    ws_ann.cell(row=r,column=5).fill=fl(bg); ws_ann.cell(row=r,column=5).border=tborder()

# Annual Overview: Budget vs Actual grouped bar chart
ann_chart_r = km + len(km_items) + 3
ann_bc = BarChart()
ann_bc.type="col"; ann_bc.style=10; ann_bc.grouping="clustered"
ann_bc.title="Annual Budget vs Actual — All Categories"
ann_bc.height=14; ann_bc.width=28
# Source: rows income_ann_row through expenses_ann_row, cols 14 (actual) + 15 (budget)
bc_cats  = Reference(ws_ann,min_col=1,min_row=income_ann_row,max_row=expenses_ann_row)
bc_act   = Reference(ws_ann,min_col=14,min_row=income_ann_row-1,max_row=expenses_ann_row)
bc_budg  = Reference(ws_ann,min_col=15,min_row=income_ann_row-1,max_row=expenses_ann_row)
ann_bc.add_data(bc_act, titles_from_data=True)
ann_bc.add_data(bc_budg,titles_from_data=True)
ann_bc.set_categories(bc_cats)
try:
    ann_bc.series[0].graphicalProperties.solidFill = SAGE_DARK
    ann_bc.series[1].graphicalProperties.solidFill = TERRA
except Exception: pass
ws_ann.add_chart(ann_bc, f"A{ann_chart_r}")

# NCF line chart
ncf_lc = LineChart()
ncf_lc.style=10; ncf_lc.title="Net Cash Flow — Monthly Trend"
ncf_lc.height=10; ncf_lc.width=20
ncf_ref  = Reference(ws_ann,min_col=2,min_row=ncf_ann_row,max_col=13,max_row=ncf_ann_row)
ncf_cats = Reference(ws_ann,min_col=2,min_row=3,max_col=13,max_row=3)
ncf_lc.add_data(ncf_ref)
ncf_lc.set_categories(ncf_cats)
try: ncf_lc.series[0].graphicalProperties.solidFill = SAGE_DARK
except Exception: pass
ws_ann.add_chart(ncf_lc, f"H{ann_chart_r}")

freeze(ws_ann,"B3")
set_widths(ws_ann,{"A":26,**{get_column_letter(2+i):10 for i in range(12)},"N":14,"O":14,"P":14})


# ═══════════════════════════════════════════════════════════════════════════
# SHEET ORDER ENFORCEMENT
# ═══════════════════════════════════════════════════════════════════════════
desired = (["Dashboard"]+MONTHS+
           ["Debt Tracker","Bill Calendar","Savings Goals","Spending Tracker","Annual Overview"])
assert wb.sheetnames == desired, f"Sheet order error: {wb.sheetnames}"

OUT = "ultimate_budget_v3.xlsx"
wb.save(OUT)
print(f"Saved: {OUT}")
print(f"Sheets ({len(wb.sheetnames)}): {', '.join(wb.sheetnames)}")
