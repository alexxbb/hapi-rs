import hou

otl = '/Users/alex/CLionProjects/hapi/otls/run_script.hda'

hou.hda.installFile(otl)

d = hou.hda.definitionsInFile(otl)[0]

nt = d.nodeType().name()

asset = hou.node('/obj/').createNode(nt)
asset.parm('script').set('/Users/alex/CLionProjects/hapi/client/one_off.py')
asset.parm('run').pressButton()