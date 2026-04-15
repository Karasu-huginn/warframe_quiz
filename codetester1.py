from random import randint
from json import loads

class Question():
    def __init__(self):
        self.dictSolution = {}
        self.reponses = ["1", "2", "3", "4"]
        self.enonce = "enonce"
        self.reponseSolution = "0"

    def obtenirQuestion(self):
        typesQuestion = ["WARFRAME","ABILITY","CHARACTER"]
        typeQuestion = typesQuestion[randint(0, len(typesQuestion)-1)]
        appelQuestion = "Question.question_"+typeQuestion
        eval(appelQuestion)(self)

    
    def question_WARFRAME(self):
        contenuChoisi = 5
        while contenuChoisi == 5 or contenuChoisi == 55:
            contenuChoisi = randint(0, 62)
        with open("warframe/warframes.txt","r") as fichier:
            for ligne, contenu in enumerate(fichier):
                if ligne == contenuChoisi:
                    self.dictSolution = loads(contenu.strip())
        fichier.close()
        specificitesQuestion = ["Abilities","Description","Icon"]
        choixSpecificiteQuestion = specificitesQuestion[randint(0, len(specificitesQuestion)-1)]
        if choixSpecificiteQuestion == "Abilities":
            self.enonce = "À quel(le) "+self.dictSolution["Type"]+" appartiennent ces capacités ?"
            Question.setReponsesPowersuitsWarframe(self)
        elif choixSpecificiteQuestion == "Description":
            self.enonce = "À quel(le) "+self.dictSolution["Type"]+" appartient cette description ?"
            Question.setReponsesPowersuitsWarframe(self)
        elif choixSpecificiteQuestion == "Icon":
            self.enonce = "À quel(le) "+self.dictSolution["Type"]+" appartient cette image ?"
            Question.setReponsesPowersuitsWarframe(self)
    
    def question_ABILITY(self):
        contenuChoisi = randint(0, 264)
        with open("warframe/abilities.txt","r") as fichier:
            for ligne, contenu in enumerate(fichier):
                if ligne == contenuChoisi:
                    self.dictSolution = loads(contenu.strip())
        fichier.close()
        specificitesQuestion = ["Powersuit","Description","Icon","Cost"]
        choixSpecificiteQuestion = specificitesQuestion[randint(0, len(specificitesQuestion)-1)]
        if choixSpecificiteQuestion == "Powersuit":
            self.enonce = "À quelle Warframe / Accessoire appartient cette capacité ?"
            Question.setReponsesPowersuitsAbility(self)
        elif choixSpecificiteQuestion == "Description":
            self.enonce = "À quelle capacité appartient cette description ?"
            Question.setReponsesAbilitiesAbility(self)
        elif choixSpecificiteQuestion == "Icon":
            self.enonce = "À quelle capacité appartient cette icône ?"
            Question.setReponsesAbilitiesAbility(self)
        else:
            self.enonce = "Combien coûte cette capacité de "+self.dictSolution["Powersuit"]+"?"
            Question.setReponsesCostAbility(self)

    def question_CHARACTER(self):
        contenuChoisi = randint(0, 160)
        with open("warframe/characters.txt","r") as fichier:
            for ligne, contenu in enumerate(fichier):
                if ligne == contenuChoisi:
                    self.dictSolution = loads(contenu.strip())
        fichier.close()
        specificitesQuestion = ["Description","Icon","Faction"]
        choixSpecificiteQuestion = specificitesQuestion[randint(0, len(specificitesQuestion)-1)]
        if choixSpecificiteQuestion == "Faction":
            self.enonce = "À quelle Faction appartient ce personnage ?"
            Question.setReponsesFactionCharacter(self)
        elif choixSpecificiteQuestion == "Description":
            self.enonce = "À quel personnage appartient cette description ?"
            Question.setReponsesCharactersCharacter(self)
        elif choixSpecificiteQuestion == "Icon":
            self.enonce = "Qui est ce personnage ?"
            Question.setReponsesCharactersCharacter(self)

    def question_MOD(self):
        pass

    def setReponsesPowersuitsWarframe(self):
        self.reponseSolution = randint(0,3)
        for i in range(4):
            if i == self.reponseSolution:
                self.reponses[i] = self.dictSolution["Name"]
            else:
                fakeDict = {"Name" : self.dictSolution["Name"],"Type" : self.dictSolution["Type"]}
                while fakeDict["Name"] == self.dictSolution["Name"] or fakeDict["Type"] != self.dictSolution["Type"]:
                    contenuChoisi = randint(0,62)
                    with open("warframe/warframes.txt","r") as fichier:
                        for ligne, contenu in enumerate(fichier):
                            if ligne == contenuChoisi:
                                fakeDict = loads(contenu.strip())
                    fichier.close()
                self.reponses[i] = fakeDict["Name"]

    def setReponsesPowersuitsAbility(self):
        self.reponseSolution = randint(0,3)
        for i in range(4):
            if i == self.reponseSolution:
                self.reponses[i] = self.dictSolution["Powersuit"]
            else:
                fakeDict = {"Powersuit" : self.dictSolution["Powersuit"]}
                while fakeDict["Powersuit"] == self.dictSolution["Powersuit"]:
                    contenuChoisi = randint(0,264)
                    with open("warframe/abilities.txt","r") as fichier:
                        for ligne, contenu in enumerate(fichier):
                            if ligne == contenuChoisi:
                                fakeDict = loads(contenu.strip())
                    fichier.close()
                self.reponses[i] = fakeDict["Powersuit"]

    def setReponsesAbilitiesAbility(self):
        self.reponseSolution = randint(0,3)
        for i in range(4):
            if i == self.reponseSolution:
                self.reponses[i] = self.dictSolution["Name"]
            else:
                fakeDict = {"Name" : self.dictSolution["Name"]}
                while fakeDict["Name"] == self.dictSolution["Name"]:
                    contenuChoisi = randint(0,264)
                    with open("warframe/abilities.txt","r") as fichier:
                        for ligne, contenu in enumerate(fichier):
                            if ligne == contenuChoisi:
                                fakeDict = loads(contenu.strip())
                    fichier.close()
                self.reponses[i] = fakeDict["Name"]

    def setReponsesCostAbility(self):
        self.reponseSolution = randint(0,3)
        for i in range(4):
            if i == self.reponseSolution:
                self.reponses[i] = self.dictSolution["Cost"]
            else:
                fakeAnswer = randint(0,20)*5
                while fakeAnswer == self.dictSolution["Cost"]:
                    fakeAnswer = randint(0,20)*5
                self.reponses[i] = fakeAnswer
    
    def setReponsesFactionCharacter(self):
        self.reponseSolution = randint(0,3)
        for i in range(4):
            if i == self.reponseSolution:
                self.reponses[i] = self.dictSolution["Faction"]
            else:
                fakeDict = {"Faction" : self.dictSolution["Faction"]}
                while fakeDict["Faction"] == self.dictSolution["Faction"]:
                    contenuChoisi = randint(0,160)
                    with open("warframe/characters.txt","r") as fichier:
                        for ligne, contenu in enumerate(fichier):
                            if ligne == contenuChoisi:
                                fakeDict = loads(contenu.strip())
                    fichier.close()
                self.reponses[i] = fakeDict["Faction"]

    def setReponsesCharactersCharacter(self):
        self.reponseSolution = randint(0,3)
        for i in range(4):
            if i == self.reponseSolution:
                self.reponses[i] = self.dictSolution["Name"]
            else:
                fakeDict = {"Name" : self.dictSolution["Name"]}
                while fakeDict["Name"] == self.dictSolution["Name"]:
                    contenuChoisi = randint(0,160)
                    with open("warframe/characters.txt","r") as fichier:
                        for ligne, contenu in enumerate(fichier):
                            if ligne == contenuChoisi:
                                fakeDict = loads(contenu.strip())
                    fichier.close()
                self.reponses[i] = fakeDict["Name"]