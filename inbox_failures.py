
from argparse import ArgumentParser
from datetime import datetime
from functools import reduce
from glob import glob
from itertools import groupby
from os import path, listdir, remove
from shutil import move
import sys
from re import compile as regex
import tkinter as tk
from types import SimpleNamespace
from tqdm import tqdm
import xlwings

SAP_SIGMANEST_PRD = r"\\hiifileserv1\sigmanestprd"
SAP_DATA_FILES = r"\\hssieng\SNData\SimTrans\SAP Data Files"

sap_archive = path.join(SAP_SIGMANEST_PRD, "Archive")
timestamp = "Production_{:%Y%m%d%H%M%S}.ready".format(datetime.now())
output_filename = path.join(SAP_DATA_FILES, "other", timestamp)

# regular expressions
INBOX_TEXT_1 = regex(r"Planned order not found for (\d{7}[a-zA-Z]-[\w-]+), (D-\d{7}-\d{5}), ([\d,]+).000, Sigmanest Program:([\d-]+)")
PROD_FILES = regex(r"Production_\d{14}.ready")
ARCHIVE_FILES = regex(r"Production_\d{14}.outbound.archive")

JOB_RE = regex(r"\d{7}[a-zA-Z]")
JOB_MARK_RE = regex(r"\d{7}[a-zA-Z]-[a-zA-Z0-9-]+")
WBS_RE = regex(r"(?:[SD]-)?(?:\d{7}?-)?(\d{5})")
PROG_RE = regex(r"\d{5}")
SPLIT_RE = regex(r"[\t, ]")

SCAN_PART = regex(r"(\d{3})([a-zA-Z])-\w+-\w+-(\w+)")
DATA_FILE_JOB = regex(r"S-(\d{7})")

# cnf file index
IPART = SimpleNamespace(matl=0, qty=4, wbs=2, plant=11, job=1, program=12)
IMATL = SimpleNamespace(matl=6, qty=8, loc=10, wbs=7, plant=11)

OUTFILE_DEFAULT = "Production_{}.ready".format(datetime.now().strftime("%Y%m%d%H%M%S"))

def part_name(_part, _job):
    scan_match = SCAN_PART.match(_part)
    job_match = DATA_FILE_JOB.match(_job)
    if not (scan_match and job_match):
        return _part

    job_end, structure, part = scan_match.groups()
    job_without_structure = job_match.group(1)

    # if _part == "252B-1B-X-M318A":
    #     print("\nmatches:", scan_match.groups(), "|", job_match.groups())
    #     print("scan match:", bool(scan_match and job_match))
    #     print("endswith match:", job_without_structure.endswith(job_end))
    #     print("partname:", "{}{}-{}".format(job_without_structure, structure, part))

    if not job_without_structure.endswith(job_end):
        return _part

    return "{}{}-{}".format(job_without_structure, structure, part)


class FailuresFinder:

    def __init__(self):
        self.run()

    def ui(self):
        args = list()

        window = tk.Tk()
        window.title("SAP Inbox Failures")

        frame = tk.Frame(window, padx=10, pady=10)
        frame.grid(row=0, column=0)

        # create checkboxes
        def toggle():
            if "--reset" in args:   args.remove("--reset")
            else:                   args.append("--reset")
        reset = tk.Checkbutton(master=frame, command=toggle, text="Remove previously generated files")

        # create buttons
        def callback(arg=None):
            if arg:
                args.append(arg)

            window.destroy()
        gen_parts   = tk.Button(master=frame, text="Generate parts list",               command=lambda: callback("--parts"))
        process     = tk.Button(master=frame, text="Generate new confirmation file",    command=lambda: callback())
        move_file   = tk.Button(master=frame, text="Move confirmation file(s)",         command=lambda: callback("--move"))

        # attach to UI
        gen_parts.grid(row=0, column=0)
        process.grid(row=1, column=0)
        move_file.grid(row=2, column=0)
        reset.grid(row=3, column=0)
        
        for child in frame.winfo_children():
            child.grid_configure(padx=5, pady=5)

        def on_quit():
            args.append("QUITTED")
            window.destroy()

        window.protocol("WM_DELETE_WINDOW", on_quit)
        window.mainloop()

        return args

    def run(self):
        ap = ArgumentParser(description="Inbox failures helper")
        ap.add_argument("-a", "--all", action="store_true", help="search all processed files")
        ap.add_argument("-m", "--max", action="store", type=int, default=200, help="max processed files to search (default: 200)")
        ap.add_argument(      "--move", action="store_true", help="move any generated Production_*.ready files")
        ap.add_argument("-n", "--name", action="store", help="save output file to tmp dir as [name]")
        ap.add_argument("-p", "--parts", action="store_true", help="only export parts list")
        ap.add_argument("-r", "--reset", action="store_true", help="remove all generated files first")

        # TODO: ui error handling

        if len(sys.argv) > 1:
            self.args = ap.parse_args()
        else:   # get args from gui
            args = self.ui()
            if "QUITTED" in args:
                return
            self.args = ap.parse_args( args )

        if self.args.reset:
            self.reset()

        if self.args.move:
            for pf in glob("Production_*.ready"):
                move(pf, r"\\hiifileserv1\sigmanestprd\Outbound")
            return

        self.failures = self.get_failures()
        
        # flatten to (Part,Program) level
        self.flatten_failures()

        if self.args.parts:
            with open("Parts.txt", 'w') as partsfile:
                parts = sorted( set([f.mark for f in self.failures]) )
                partsfile.write("\n".join(parts))
            return

        self.get_data_rows()
        if self.apply_planned_orders():
            self.output()

    def reset(self):
        generated_files = [
            "NoCnfRowFound.txt",
            "Parts.txt",
            "NoPlannedOrderFound.txt"
        ]

        for gen_file in generated_files:
            try:
                remove(gen_file)
            except FileNotFoundError:
                pass

    def get_failures(self):
        to_find = list()

        print("Parsing inbox...")
        with open("inbox.txt", "r") as inbox:
            for line in inbox.readlines():
                try:
                    to_find.append( Failure(line) )
                except RuntimeError:
                    print("Error parsing line:", line)

        return to_find

    def get_data_rows(self):
        parser = CnfFileParser()
        found = list()

        files = self.files
        for processed_file in tqdm(files, desc="Fetching Data", total=len(files)):
            for line in parser.parse_file( processed_file ):
                index = self.find_failure(line)
                if index > -1:    # in self.failures
                    failure = self.failures.pop(index)
                    failure.apply_cnf_row(line)
                    found.append( failure )

            # stop if no failures left
            if len( self.failures ) == 0:
                break


        if len( self.failures ) > 0:
            with open("NoCnfRowFound.txt", 'w') as f:
                for failure in self.failures:
                    f.write(failure.line + "\n")

        self.failures = found

    def apply_planned_orders(self):
        try:
            wb = xlwings.books.active
        except xlwings.XlwingsError:
            return False


        print("Applying planned orders...")

        sheet = wb.sheets.active
        header = sheet.range("A1").expand('right').value
        for i, col in enumerate(header):
            if col == "Material Number":
                imatl = i
            elif col == "Order quantity (GMEIN)":
                iqty = i
            elif col == "WBS Element":
                iwbs = i
            elif col == "Plant":
                iplant = i

        assert None not in (imatl, iqty, iwbs, iplant), "COHV header not matched completely"

        for row in sheet.range("A2").expand().options(ndim=2).value:
            part = row[imatl]
            wbs = row[iwbs]
            qty = row[iqty]
            plant = row[iplant]

            for tc in self.failures:
                if qty == 0:
                    break

                if tc.mark == part:
                    qty = tc.apply_wbs(wbs, qty, plant)

        return True

    def flatten_failures(self):
        # sort and group failures
        grouped = groupby(sorted(self.failures))

        # reduce grouped items (accumulates qty and area)
        self.failures = [ reduce(lambda a,b: a+b, list(v)) for _, v in grouped ]

    def output(self):
        print("Generating output...")

        not_confirmed = list()
        with open(OUTFILE_DEFAULT, 'w') as outfile:
            for failure in sorted(self.failures):
                for output in failure.output():
                    outfile.write( "\t".join(output) )

                if failure.qty > 0:
                    not_confirmed.append(failure)


        if len(not_confirmed) > 0:
            with open("NoPlannedOrderFound.txt", 'w') as ncfile:
                ncfile.write("Mark,Qty,Wbs,Program\n")
                for f in not_confirmed:
                    ncfile.write("{},{},{},{}\n".format(f.mark, f.qty, f.wbs, f.prog))
                    

    @property
    def files(self):
        # get files matching expected file name pattern, in reverse order (newest first)
        def get_files(folder, file_re):
            return sorted([path.join(folder, f) for f in listdir(folder) if file_re.match(f)], reverse=True)

        files = get_files(sap_archive, ARCHIVE_FILES)

        if self.args.all:
            return files

        # IDEA: once a part is found, we have likely found a neigborhood for parts from the same job
        return files[:self.args.max]

    # used to see if a CnfFileRow is in the list of Failures
    def find_failure(self, cnf_row):
        for i, failure in enumerate(self.failures):
            if failure.matches_cnf_row( cnf_row ):
                return i

        return -1
        

class Failure:

    def __init__(self, line):
        self.line = line.strip()

        match = INBOX_TEXT_1.match(self.line)
        if match:
            self.mark   = match.group(1)
            self.wbs    = match.group(2)
            self.qty    = int(match.group(3).replace(",", ""))
            self.prog   = match.group(4)

        else:
            raise RuntimeError("Line could not be parsed")

        self.applied_cnf_row = None
        self.applied_wbs_elems = list()

    def apply_cnf_row(self, cnf_row):
        self.applied_cnf_row = cnf_row

    def apply_wbs(self, wbs, qty, plant):
        if self.qty == 0:
            return qty

        if qty <= self.qty:
            self.applied_wbs_elems.append((wbs, qty, plant))
            self.qty -= qty
            return 0

        else:
            self.applied_wbs_elems.append((wbs, self.qty, plant))
            remaining = qty - self.qty
            self.qty = 0
            return remaining

    def output(self):
        for wbs, qty, plant in self.applied_wbs_elems:
            yield self.applied_cnf_row.output(wbs, qty, plant)

    def __repr__(self):
        return "<Failure> {} ({}) {} {}".format(self.mark, self.qty, self.wbs, self.prog)

    def __eq__(self, other):
        # for effeciency, we go in order or smallest qualifier to largest
        # program -> part
        if self.prog == other.prog:
            if self.mark == other.mark:
                return True

        return False

    def matches_cnf_row(self, cnf_row):
        # for effeciency, we go in order or smallest qualifier to largest
        # program -> part -> wbs
        if self.prog == cnf_row.program:
            if self.mark == cnf_row.part_name:
                if self.wbs == cnf_row.part_wbs:
                    return True

        return False

    def __lt__(self, other):
        # make sort order (mark, prog, wbs)
        return (self.mark, self.prog, self.wbs) < (other.mark, other.prog, other.wbs)

    def __add__(self, other):
        assert (self.mark, self.prog) == (other.mark, other.prog), "Cannot add Failures of differing mark and program"

        self.qty += other.qty

        return self


class CnfFileParser:

    def parse_file(self, filename):
        with open(filename, "r") as prod_file:
            for line in prod_file.readlines():
                try:
                    yield CnfFileRow( line.upper().strip().split("\t") )
                except Exception:
                    pass


class CnfFileRow:

    def __init__(self, row):
        
        # part
        self.part_name = row[IPART.matl]
        self.part_job  = row[IPART.job]
        self.part_wbs  = row[IPART.wbs]
        self.part_qty  = int(row[IPART.qty])

        # material
        self.matl_master = row[IMATL.matl]
        self.matl_wbs    = row[IMATL.wbs]
        self.matl_qty    = float(row[IMATL.qty])
        self.matl_loc    = row[IMATL.loc]
        self.matl_plant  = row[IMATL.plant]

        self.program = row[IPART.program]

    @property
    def matl_qty_per_ea(self):
        return self.matl_qty / self.part_qty

    def output(self, wbs, qty, plant):
        return [
            #part
            self.part_name,
            self.part_job,
            wbs,
            "PROD",
            str(int(qty)),
            "EA",
            
            # material
            self.matl_master,
            self.matl_wbs,
            "{:.3f}".format( self.matl_qty_per_ea * qty ),
            "IN2",
            self.matl_loc,
            plant,
            
            self.program, "\n"
        ]


if __name__ == "__main__":
    # init calls run(), so this will run the failures finder script
    FailuresFinder()
