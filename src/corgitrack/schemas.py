from datetime import datetime

from pydantic import BaseModel


class DueDose(BaseModel):
    dose_id: int
    dog_name: str
    treatment_name: str
    dose_label: str | None
    due_at: datetime
    instructions: str | None
