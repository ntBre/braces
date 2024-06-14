# braces
fix your SMILES

## Usage
Currently the only fix supported is remapping atom numbers, which the binary
does in a loop from stdin. For example, a session (with `> ` added before input)
might look like:

``` shell
cargo run
> t61g [C:3]([N+:4]1([C:5]([C:6]([H:16])([H:17])[H:18])([H:14])[H:15])[C:7]([H:19])([H:20])[C:8]1([H:21])[H:22])([H:12])[H:13] (2, 3, 6, 18)
t61g [C:1]([N+:2]1([C:3]([C:4]([H:11])([H:12])[H:13])([H:9])[H:10])[C:5]([H:14])([H:15])[C:6]1([H:16])[H:17])([H:7])[H:8] (0, 1, 4, 13)
```

The initial SMILES has had part of it deleted, leaving the lowest index at 3 and
the highest index at 22, despite now having only 17 atoms. The tuple after the
mapped SMILES is remapped too. The OpenFF parameter ID in the first column is
ignored but passed through into the output for ease of copy-pasting. This is
very useful for modifying mapped SMILES strings by hand. I like to use Python
code like the following in a notebook to verify that the transformations
preserve the correct parameter match.

``` python
import re

from openff.toolkit import ForceField, Molecule
from rdkit import Chem
from rdkit.Chem.Draw import MolsToGridImage, rdDepictor, rdMolDraw2D

def mol_to_svg(mol: Molecule, hl_atoms) -> list[str]:
    mol = mol.to_rdkit()
    rdDepictor.SetPreferCoordGen(True)
    rdDepictor.Compute2DCoords(mol)
    rdmol = rdMolDraw2D.PrepareMolForDrawing(mol)

    return MolsToGridImage(
        [rdmol],
        useSVG=True,
        highlightAtomLists=[hl_atoms],
        subImgSize=(300, 300),
        molsPerRow=1,
    )


junk = re.compile("[(,)]")

ff = ForceField("openff-2.2.0.offxml")

while (line := input()) != "q":
    pid, smiles, *rest = line.split()
    tors = tuple([int(junk.sub("", x)) for x in rest])
    try:
        mol = Molecule.from_mapped_smiles(smiles, allow_undefined_stereo=True)
        labels = ff.label_molecules(mol.to_topology())[0]["ProperTorsions"]
        labels = {k: p.id for k, p in labels.items()}
        if labels[tors] == pid:
            print("matched!")
        else:
            print(labels)
        display(mol_to_svg(mol, tors))
    except Exception as e:
        print(e)
```

